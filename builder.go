package main

import (
	"errors"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/samber/lo"
)

type option = discordgo.ApplicationCommandOption

func getUnitOption(description string, required bool, units []*systemctlUnit) option {
	return option{
		Name:        "unit",
		Type:        discordgo.ApplicationCommandOptionString,
		Description: description,
		Required:    required,
		Choices: lo.Map(units, func(unit *systemctlUnit, _ int) *discordgo.ApplicationCommandOptionChoice {
			name := unit.Name
			return &discordgo.ApplicationCommandOptionChoice{
				Name:  strings.TrimSuffix(name, ".service"),
				Value: name,
			}
		}),
	}
}

func buildCommands(units []*systemctlUnit, callback func(name, description string, options []*option)) {
	startableUnits := getUnitsWithPermissions(units, Start)
	if len(startableUnits) > 0 {
		unitOption := getUnitOption("The unit to start", true, startableUnits)
		callback("start", "Start units", []*option{&unitOption})
	}

	stoppableUnits := getUnitsWithPermissions(units, Stop)
	if len(stoppableUnits) > 0 {
		unitOption := getUnitOption("The unit to stop", true, stoppableUnits)
		callback("stop", "Stops units", []*option{&unitOption})
	}

	restartableUnits := getUnitsWithPermissions(units, Start, Stop)
	if len(restartableUnits) > 0 {
		unitOption := getUnitOption("The unit to restart", true, restartableUnits)
		callback("restart", "Restarts units", []*option{&unitOption})
	}

	checkableUnits := getUnitsWithPermissions(units, Status)
	if len(checkableUnits) > 0 {
		unitOption := getUnitOption("The unit to check", false, checkableUnits)
		callback("status", "Check units' status", []*option{&unitOption})
	}
}

func getCommands(units []*systemctlUnit, commandType commandType) ([]*discordgo.ApplicationCommand, error) {
	switch commandType {
	case Single:
		subCommands := make([]*option, 0)
		buildCommands(units, func(name, description string, options []*option) {
			subCommands = append(subCommands, &option{
				Name:        name,
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
		buildCommands(units, func(name, description string, options []*option) {
			commands = append(commands, &discordgo.ApplicationCommand{
				Name:        name,
				Description: description,
				Options:     options,
			})
		})
		return commands, nil
	default:
		return nil, errors.New("invalid command type")
	}
}
