package handler

import (
	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

func getCommandData(data *discordgo.ApplicationCommandInteractionData) (string, []*discordgo.ApplicationCommandInteractionDataOption) {
	if data.Name == "systemctl" {
		subCommand := data.Options[0]
		return subCommand.Name, subCommand.Options
	} else {
		return data.Name, data.Options
	}
}

func AddHandler(session DiscordSession, systemd Systemd, c *config.Config) {
	commandByName := make(map[string]*Command)
	for _, cmd := range Commands {
		commandByName[cmd.Name] = cmd
	}

	session.AddHandler(func(s *discordgo.Session, event *discordgo.InteractionCreate) {
		if event.Type != discordgo.InteractionApplicationCommand {
			return
		}
		data := event.ApplicationCommandData()
		commandName, options := getCommandData(&data)
		if cmd := commandByName[commandName]; cmd != nil {
			cmd.Handle(&HandleContext{
				CommandName: commandName,
				Options:     options,
				Session:     session,
				Interaction: event.Interaction,
				Systemd:     systemd,
				Config:      c,
			})
		}
	})
}
