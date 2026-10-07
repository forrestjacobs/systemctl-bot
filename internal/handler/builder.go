package handler

import (
	"errors"
	"strconv"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

type option = discordgo.ApplicationCommandOption

type DiscordCommandSession interface {
	ApplicationCommandBulkOverwrite(appID string, guildID string, commands []*discordgo.ApplicationCommand, options ...discordgo.RequestOption) (createdCommands []*discordgo.ApplicationCommand, err error)
}

func getCommands(c *config.Config) ([]*discordgo.ApplicationCommand, error) {
	switch c.CommandType {
	case config.Single:
		subCommands := make([]*option, 0)
		for _, cmd := range Commands {
			options, applicable := cmd.Build(c)
			if applicable {
				subCommands = append(subCommands, &option{
					Name:        cmd.Name,
					Description: cmd.Description,
					Type:        discordgo.ApplicationCommandOptionSubCommand,
					Options:     options,
				})
			}
		}
		return []*discordgo.ApplicationCommand{
			{
				Name:        "systemctl",
				Description: "Controls units",
				Options:     subCommands,
			},
		}, nil
	case config.Multiple:
		commands := make([]*discordgo.ApplicationCommand, 0)
		for _, cmd := range Commands {
			options, applicable := cmd.Build(c)
			if applicable {
				commands = append(commands, &discordgo.ApplicationCommand{
					Name:        cmd.Name,
					Description: cmd.Description,
					Options:     options,
				})
			}
		}
		return commands, nil
	default:
		return nil, errors.New("invalid command type")
	}
}

func RegisterCommands(discord DiscordCommandSession, c *config.Config) error {
	applicationID := strconv.FormatUint(c.ApplicationID, 10)
	guildID := strconv.FormatUint(c.GuildID, 10)
	commands, err := getCommands(c)
	if err != nil {
		return err
	}
	_, err = discord.ApplicationCommandBulkOverwrite(applicationID, guildID, commands)
	return err
}
