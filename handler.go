package main

import (
	"github.com/bwmarrin/discordgo"
)

type discordSession interface {
	InteractionRespond(interaction *discordgo.Interaction, resp *discordgo.InteractionResponse, options ...discordgo.RequestOption) error
	FollowupMessageCreate(interaction *discordgo.Interaction, wait bool, data *discordgo.WebhookParams, options ...discordgo.RequestOption) (*discordgo.Message, error)
}

type commandCtx struct {
	commandName string
	options     []*discordgo.ApplicationCommandInteractionDataOption
	session     discordSession
	interaction *discordgo.Interaction
}

type commandRunner interface {
	run(ctx *commandCtx)
}

func getCommandData(data *discordgo.ApplicationCommandInteractionData) (string, []*discordgo.ApplicationCommandInteractionDataOption) {
	if data.Name == "systemctl" {
		subCommand := data.Options[0]
		return subCommand.Name, subCommand.Options
	} else {
		return data.Name, data.Options
	}
}

func makeInteractionHandler(runner commandRunner) func(session discordSession, event *discordgo.InteractionCreate) {
	return func(session discordSession, event *discordgo.InteractionCreate) {
		if event.Type != discordgo.InteractionApplicationCommand {
			return
		}
		data := event.ApplicationCommandData()
		commandName, options := getCommandData(&data)
		runner.run(&commandCtx{
			commandName: commandName,
			options:     options,
			session:     session,
			interaction: event.Interaction,
		})
	}
}
