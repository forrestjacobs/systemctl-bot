package main

import (
	"testing"

	"github.com/bwmarrin/discordgo"
	"github.com/go-test/deep"
	"github.com/samber/lo"
)

func makeUnit(name string, permissions ...unitPermission) *systemctlUnit {
	return &systemctlUnit{
		Name:        name + ".service",
		Permissions: permissions,
	}
}

func getUnits() []*systemctlUnit {
	return []*systemctlUnit{
		makeUnit("000"),
		makeUnit("001", Status),
		makeUnit("010", Stop),
		makeUnit("011", Stop, Status),
		makeUnit("100", Start),
		makeUnit("101", Start, Status),
		makeUnit("110", Start, Stop),
		makeUnit("111", Start, Stop, Status),
	}
}

func makeCommand(name, description string, options ...*discordgo.ApplicationCommandOption) *discordgo.ApplicationCommand {
	return &discordgo.ApplicationCommand{
		Name:        name,
		Description: description,
		Options:     options,
	}
}

func makeSubcommand(name, description string, options ...*discordgo.ApplicationCommandOption) *discordgo.ApplicationCommandOption {
	return &discordgo.ApplicationCommandOption{
		Name:        name,
		Description: description,
		Type:        discordgo.ApplicationCommandOptionSubCommand,
		Options:     options,
	}
}

func makeUnitOption(description string, required bool, units ...string) *discordgo.ApplicationCommandOption {
	return &discordgo.ApplicationCommandOption{
		Name:        "unit",
		Type:        discordgo.ApplicationCommandOptionString,
		Description: description,
		Required:    required,
		Choices: lo.Map(units, func(name string, _ int) *discordgo.ApplicationCommandOptionChoice {
			return &discordgo.ApplicationCommandOptionChoice{
				Name:  name,
				Value: name + ".service",
			}
		}),
	}
}

func TestGetSingleCommand(t *testing.T) {
	commands, err := getCommands(getUnits(), Single)

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}

	if diff := deep.Equal(commands, []*discordgo.ApplicationCommand{
		makeCommand("systemctl", "Controls units",
			makeSubcommand("start", "Start units", makeUnitOption("The unit to start", true, "100", "101", "110", "111")),
			makeSubcommand("stop", "Stop units", makeUnitOption("The unit to stop", true, "010", "011", "110", "111")),
			makeSubcommand("restart", "Restart units", makeUnitOption("The unit to restart", true, "110", "111")),
			makeSubcommand("status", "Check units' status", makeUnitOption("The unit to check", false, "001", "011", "101", "111")),
		),
	}); diff != nil {
		t.Error(diff)
	}

}

func TestMultipleCommands(t *testing.T) {
	commands, err := getCommands(getUnits(), Multiple)

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}

	if diff := deep.Equal(commands, []*discordgo.ApplicationCommand{
		makeCommand("start", "Start units", makeUnitOption("The unit to start", true, "100", "101", "110", "111")),
		makeCommand("stop", "Stop units", makeUnitOption("The unit to stop", true, "010", "011", "110", "111")),
		makeCommand("restart", "Restart units", makeUnitOption("The unit to restart", true, "110", "111")),
		makeCommand("status", "Check units' status", makeUnitOption("The unit to check", false, "001", "011", "101", "111")),
	}); diff != nil {
		t.Error(diff)
	}
}

func TestInvalidCommandType(t *testing.T) {
	_, err := getCommands(getUnits(), "invalid")

	if err.Error() != "invalid command type" {
		t.Fatalf("Unexpected error %v", err)
	}
}
