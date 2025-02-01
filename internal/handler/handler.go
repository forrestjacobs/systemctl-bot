package handler

import (
	"context"
	"log"
	"slices"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

type DiscordSession interface {
	AddHandler(handler interface{}) func()
	InteractionRespond(interaction *discordgo.Interaction, resp *discordgo.InteractionResponse, options ...discordgo.RequestOption) error
	FollowupMessageCreate(interaction *discordgo.Interaction, wait bool, data *discordgo.WebhookParams, options ...discordgo.RequestOption) (*discordgo.Message, error)
}

type Systemd interface {
	StartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	StopUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	RestartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	GetUnitPropertyContext(ctx context.Context, unit string, propertyName string) (*dbus.Property, error)
}

type commandCtx struct {
	commandName string
	options     []*discordgo.ApplicationCommandInteractionDataOption
	session     DiscordSession
	interaction *discordgo.Interaction
}

type commandRunner struct {
	systemd Systemd
	units   map[config.Command][]string
}

func (runner *commandRunner) checkAllowed(command config.Command, value string) bool {
	allowed := slices.Contains(runner.units[command], value)
	if !allowed {
		log.Println(string(command) + " is not an allowed command for " + value)
	}
	return allowed
}

func getCommandData(data *discordgo.ApplicationCommandInteractionData) (string, []*discordgo.ApplicationCommandInteractionDataOption) {
	if data.Name == "systemctl" {
		subCommand := data.Options[0]
		return subCommand.Name, subCommand.Options
	} else {
		return data.Name, data.Options
	}
}

func AddHandler(session DiscordSession, systemd Systemd, c *config.Config) {
	runner := &commandRunner{
		systemd: systemd,
		units:   c.Units,
	}
	session.AddHandler(func(s *discordgo.Session, event *discordgo.InteractionCreate) {
		if event.Type != discordgo.InteractionApplicationCommand {
			return
		}
		data := event.ApplicationCommandData()
		commandName, options := getCommandData(&data)
		if handler, ok := commandHandlers[config.Command(commandName)]; ok {
			handler(&commandCtx{
				commandName: commandName,
				options:     options,
				session:     session,
				interaction: event.Interaction,
			}, runner)
		}
	})
}
