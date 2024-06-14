package main

import (
	"context"
	"log"
	"slices"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/samber/lo"
)

func logError(err error) {
	if err != nil {
		log.Println(err)
	}
}

type discordSession interface {
	InteractionRespond(interaction *discordgo.Interaction, resp *discordgo.InteractionResponse, options ...discordgo.RequestOption) error
	FollowupMessageCreate(interaction *discordgo.Interaction, wait bool, data *discordgo.WebhookParams, options ...discordgo.RequestOption) (*discordgo.Message, error)
}

type systemd interface {
	StartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	StopUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	RestartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	GetUnitPropertyContext(ctx context.Context, unit string, propertyName string) (*dbus.Property, error)
}

type handlerCtx struct {
	systemd      systemd
	session      discordSession
	interaction  *discordgo.Interaction
	commandUnits map[command][]string
}

func (ctx *handlerCtx) respond(content string) {
	err := ctx.session.InteractionRespond(ctx.interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
		},
	})
	logError(err)
}

func (ctx *handlerCtx) deferResponse() bool {
	err := ctx.session.InteractionRespond(ctx.interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	})
	logError(err)
	return err == nil
}

func (ctx *handlerCtx) followUp(content string) {
	_, err := ctx.session.FollowupMessageCreate(ctx.interaction, false, &discordgo.WebhookParams{
		Content: content,
	})
	logError(err)
}

func (ctx *handlerCtx) checkAllowed(command command, value string) bool {
	allowed := slices.Contains(ctx.commandUnits[command], value)
	if !allowed {
		ctx.respond("command is not allowed")
	}
	return allowed
}

func getSystemdResponse(doneString string, resultChan <-chan string, err error) string {
	if err != nil {
		return err.Error()
	}

	result := <-resultChan
	if result == "done" {
		return doneString
	}

	return result
}

var commandHandlers = map[command]func(ctx *handlerCtx, options []*discordgo.ApplicationCommandInteractionDataOption){
	StartCommand: func(ctx *handlerCtx, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if ctx.checkAllowed(StartCommand, unit) && ctx.deferResponse() {
			resultChan := make(chan string)
			_, err := ctx.systemd.StartUnitContext(context.Background(), unit, "replace", resultChan)
			ctx.followUp(getSystemdResponse("Started "+unit, resultChan, err))
		}
	},

	StopCommand: func(ctx *handlerCtx, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if ctx.checkAllowed(StopCommand, unit) && ctx.deferResponse() {
			resultChan := make(chan string)
			_, err := ctx.systemd.StopUnitContext(context.Background(), unit, "replace", resultChan)
			ctx.followUp(getSystemdResponse("Stopped "+unit, resultChan, err))
		}
	},

	RestartCommand: func(ctx *handlerCtx, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if ctx.checkAllowed(RestartCommand, unit) && ctx.deferResponse() {
			resultChan := make(chan string)
			_, err := ctx.systemd.RestartUnitContext(context.Background(), unit, "replace", resultChan)
			ctx.followUp(getSystemdResponse("Restarted "+unit, resultChan, err))
		}
	},

	StatusCommand: func(ctx *handlerCtx, options []*discordgo.ApplicationCommandInteractionDataOption) {
		if len(options) == 0 {
			lines := lo.FilterMap(ctx.commandUnits[StatusCommand], func(unit string, _ int) (string, bool) {
				prop, err := ctx.systemd.GetUnitPropertyContext(context.Background(), unit, "ActiveState")
				if err != nil {
					log.Println("Error fetching unit state: ", err)
					return unit + ": error getting status", true
				}
				val := prop.Value.Value().(string)
				return unit + ": " + val, val != "inactive"
			})

			if len(lines) == 0 {
				ctx.respond("Nothing is active")
			} else {
				ctx.respond(strings.Join(lines, "\n"))
			}
		} else {
			unit := options[0].StringValue()
			if ctx.checkAllowed(StatusCommand, unit) {
				prop, err := ctx.systemd.GetUnitPropertyContext(context.Background(), unit, "ActiveState")
				if err != nil {
					ctx.respond(err.Error())
				} else {
					ctx.respond(prop.Value.Value().(string))
				}
			}
		}
	},
}

func getCommandData(data *discordgo.ApplicationCommandInteractionData) (string, []*discordgo.ApplicationCommandInteractionDataOption) {
	if data.Name == "systemctl" {
		subCommand := data.Options[0]
		return subCommand.Name, subCommand.Options
	} else {
		return data.Name, data.Options
	}
}

func makeInteractionHandler(commandUnits map[command][]string, systemd systemd) func(session discordSession, event *discordgo.InteractionCreate) {
	return func(session discordSession, event *discordgo.InteractionCreate) {
		if event.Type != discordgo.InteractionApplicationCommand {
			return
		}
		data := event.ApplicationCommandData()
		name, options := getCommandData(&data)
		if handler, ok := commandHandlers[command(name)]; ok {
			handler(&handlerCtx{
				commandUnits: commandUnits,
				systemd:      systemd,
				session:      session,
				interaction:  event.Interaction,
			}, options)
		}
	}
}
