package main

import (
	"log"
	"slices"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/samber/lo"
)

func logError(err error) {
	if err != nil {
		log.Println(err)
	}
}

type interaction interface {
	getSystemd() systemd
	getUnits(command command) []string
	respond(content string)
	deferResponse() bool
	followUp(content string)
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

func (i *interactionStruct) respond(content string) {
	err := i.session.InteractionRespond(i.interaction.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
		},
	})
	logError(err)
}

func (i *interactionStruct) deferResponse() bool {
	err := i.session.InteractionRespond(i.interaction.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	})
	logError(err)
	return err == nil
}

func (i *interactionStruct) followUp(content string) {
	_, err := i.session.FollowupMessageCreate(i.interaction.Interaction, false, &discordgo.WebhookParams{
		Content: content,
	})
	logError(err)
}

func checkAllowed(i interaction, command command, value string) bool {
	allowed := slices.Contains(i.getUnits(command), value)
	if !allowed {
		i.respond("command is not allowed")
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

var commandHandlers = map[command]func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption){
	StartCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if checkAllowed(i, StartCommand, unit) && i.deferResponse() {
			resultChan, err := i.getSystemd().start(unit)
			i.followUp(getSystemdResponse("Started "+unit, resultChan, err))
		}
	},

	StopCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if checkAllowed(i, StopCommand, unit) && i.deferResponse() {
			resultChan, err := i.getSystemd().stop(unit)
			i.followUp(getSystemdResponse("Stopped "+unit, resultChan, err))
		}
	},

	RestartCommand: func(i interaction, options []*discordgo.ApplicationCommandInteractionDataOption) {
		unit := options[0].StringValue()
		if checkAllowed(i, RestartCommand, unit) && i.deferResponse() {
			resultChan, err := i.getSystemd().restart(unit)
			i.followUp(getSystemdResponse("Restarted "+unit, resultChan, err))
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
				activeState, err := i.getSystemd().getUnitActiveState(unit)
				if err != nil {
					i.respond(err.Error())
				} else {
					i.respond(activeState)
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
