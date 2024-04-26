package main

import (
	"log"
	"slices"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/samber/lo"
)

type interaction interface {
	getSystemd() systemd
	getUnits(command command) []string
	respond(content string) error
	deferResponse() error
	followUp(content string) error
}

type interactionStruct struct {
	systemd      systemd
	commandUnits map[command][]string
	session      *discordgo.Session
	interaction  *discordgo.InteractionCreate
}

func (i *interactionStruct) getSystemd() systemd {
	return i.systemd
}

func (i *interactionStruct) getUnits(command command) []string {
	return i.commandUnits[command]
}

func (i *interactionStruct) respond(content string) error {
	return i.session.InteractionRespond(i.interaction.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
		},
	})
}

func (i *interactionStruct) deferResponse() error {
	return i.session.InteractionRespond(i.interaction.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	})
}

func (i *interactionStruct) followUp(content string) error {
	_, err := i.session.FollowupMessageCreate(i.interaction.Interaction, false, &discordgo.WebhookParams{
		Content: content,
	})
	return err
}

func checkAllowed(i interaction, command command, value string) bool {
	allowed := slices.Contains(i.getUnits(command), value)
	if !allowed {
		i.respond("command is not allowed")
	}
	return allowed
}

func getContent(success string, err error) string {
	if err != nil {
		return err.Error()
	} else {
		return success
	}
}

var commandHandlers = map[command]func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption){
	StartCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if checkAllowed(i, StartCommand, unit) && i.deferResponse() == nil {
			err := i.getSystemd().start(unit)
			i.followUp(getContent("Started "+unit, err))
		}
	},

	StopCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if checkAllowed(i, StopCommand, unit) && i.deferResponse() == nil {
			err := i.getSystemd().stop(unit)
			i.followUp(getContent("Stopped "+unit, err))
		}
	},

	RestartCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if checkAllowed(i, RestartCommand, unit) && i.deferResponse() == nil {
			err := i.getSystemd().restart(unit)
			i.followUp(getContent("Restarted "+unit, err))
		}
	},

	StatusCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		if len(options) == 0 {
			lines := lo.FilterMap(i.getUnits(StatusCommand), func(unit string, _ int) (string, bool) {
				val, err := i.getSystemd().getUnitActiveState(unit)
				if err != nil {
					log.Println("Error fetching unit state: ", err)
					return unit + ": error getting status", true
				}
				return unit + ": " + val, val != "inactive"
			})

			if len(lines) == 0 {
				i.respond("Nothing is active")
			} else {
				i.respond(strings.Join(lines, "\n"))
			}
		} else {
			unit := options[0].StringValue()
			if checkAllowed(i, StatusCommand, unit) {
				i.respond(getContent(i.getSystemd().getUnitActiveState(unit)))
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

func makeInteractionHandler(commandUnits map[command][]string, systemd systemd) func(session *discordgo.Session, i *discordgo.InteractionCreate) {
	return func(session *discordgo.Session, interaction *discordgo.InteractionCreate) {
		if interaction.Type != discordgo.InteractionApplicationCommand {
			return
		}
		data := interaction.ApplicationCommandData()
		name, options := getCommandData(&data)
		if h, ok := commandHandlers[command(name)]; ok {
			h(&interactionStruct{
				commandUnits: commandUnits,
				systemd:      systemd,
				session:      session,
				interaction:  interaction,
			}, options)
		}
	}
}
