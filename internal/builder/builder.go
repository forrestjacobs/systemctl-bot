package builder

import (
	"errors"
	"strconv"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

type option = discordgo.ApplicationCommandOption

func getUnitOption(description string, units []string) option {
	choices := []*discordgo.ApplicationCommandOptionChoice{}
	for _, unit := range units {
		choices = append(choices, &discordgo.ApplicationCommandOptionChoice{
			Name:  strings.TrimSuffix(unit, ".service"),
			Value: unit,
		})
	}
	return option{
		Name:        "unit",
		Type:        discordgo.ApplicationCommandOptionString,
		Description: description,
		Required:    true,
		Choices:     choices,
	}
}

func buildCommands(units map[config.Command][]string, callback func(name config.Command, description string, options []*option)) {
	if len(units[config.StartCommand]) > 0 {
		unitOption := getUnitOption("The unit to start", units[config.StartCommand])
		callback(config.StartCommand, "Start units", []*option{&unitOption})
	}

	if len(units[config.StopCommand]) > 0 {
		unitOption := getUnitOption("The unit to stop", units[config.StopCommand])
		callback(config.StopCommand, "Stop units", []*option{&unitOption})
	}

	if len(units[config.RestartCommand]) > 0 {
		unitOption := getUnitOption("The unit to restart", units[config.RestartCommand])
		callback(config.RestartCommand, "Restart units", []*option{&unitOption})
	}
}

func getCommands(units map[config.Command][]string, commandType config.CommandType) ([]*discordgo.ApplicationCommand, error) {
	switch commandType {
	case config.Single:
		subCommands := make([]*option, 0)
		buildCommands(units, func(name config.Command, description string, options []*option) {
			subCommands = append(subCommands, &option{
				Name:        string(name),
				Description: description,
				Type:        discordgo.ApplicationCommandOptionSubCommand,
				Options:     options,
			})
		})
		return []*discordgo.ApplicationCommand{
			{
				Name:        "systemctl",
				Description: "Controls units",
				Options:     subCommands,
			},
		}, nil
	case config.Multiple:
		commands := make([]*discordgo.ApplicationCommand, 0)
		buildCommands(units, func(name config.Command, description string, options []*option) {
			commands = append(commands, &discordgo.ApplicationCommand{
				Name:        string(name),
				Description: description,
				Options:     options,
			})
		})
		return commands, nil
	default:
		return nil, errors.New("invalid command type")
	}
}

type DiscordSession interface {
	ApplicationCommandBulkOverwrite(appID string, guildID string, commands []*discordgo.ApplicationCommand, options ...discordgo.RequestOption) (createdCommands []*discordgo.ApplicationCommand, err error)
}

func RegisterCommands(discord DiscordSession, c *config.Config) error {
	applicationID := strconv.FormatUint(c.ApplicationID, 10)
	guildID := strconv.FormatUint(c.GuildID, 10)
	commands, err := getCommands(c.CommandUnits, c.CommandType)
	if err != nil {
		return err
	}
	_, err = discord.ApplicationCommandBulkOverwrite(applicationID, guildID, commands)
	return err
}
