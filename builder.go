package main

import (
	"errors"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/samber/lo"
)

type option = discordgo.ApplicationCommandOption

func getUnitOption(description string, required bool, units []string) option {
	return option{
		Name:        "unit",
		Type:        discordgo.ApplicationCommandOptionString,
		Description: description,
		Required:    required,
		Choices: lo.Map(units, func(unit string, _ int) *discordgo.ApplicationCommandOptionChoice {
			return &discordgo.ApplicationCommandOptionChoice{
				Name:  strings.TrimSuffix(unit, ".service"),
				Value: unit,
			}
		}),
	}
}

func buildCommands(commandUnits map[command][]string, callback func(name command, description string, options []*option)) {
	if len(commandUnits[StartCommand]) > 0 {
		unitOption := getUnitOption("The unit to start", true, commandUnits[StartCommand])
		callback(StartCommand, "Start units", []*option{&unitOption})
	}

	if len(commandUnits[StopCommand]) > 0 {
		unitOption := getUnitOption("The unit to stop", true, commandUnits[StopCommand])
		callback(StopCommand, "Stop units", []*option{&unitOption})
	}

	if len(commandUnits[RestartCommand]) > 0 {
		unitOption := getUnitOption("The unit to restart", true, commandUnits[RestartCommand])
		callback(RestartCommand, "Restart units", []*option{&unitOption})
	}

	if len(commandUnits[StatusCommand]) > 0 {
		unitOption := getUnitOption("The unit to check", false, commandUnits[StatusCommand])
		callback(StatusCommand, "Check units' status", []*option{&unitOption})
	}
}

func getCommands(commandUnits map[command][]string, commandType commandType) ([]*discordgo.ApplicationCommand, error) {
	switch commandType {
	case Single:
		subCommands := make([]*option, 0)
		buildCommands(commandUnits, func(name command, description string, options []*option) {
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
	case Multiple:
		commands := make([]*discordgo.ApplicationCommand, 0)
		buildCommands(commandUnits, func(name command, description string, options []*option) {
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
