package main

import (
	"reflect"
	"testing"

	"github.com/bwmarrin/discordgo"
	"github.com/samber/lo"
)

func getBuilderTestCommandUnits() map[command][]string {
	return map[command][]string{
		StartCommand:   {"startable.service", "restartable.service"},
		StopCommand:    {"stoppable.service", "restartable.service"},
		RestartCommand: {"restartable.service"},
		StatusCommand:  {"startable.service", "stoppable.service", "restartable.service"},
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
	commands, err := getCommands(getBuilderTestCommandUnits(), Single)

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}

	if !reflect.DeepEqual(commands, []*discordgo.ApplicationCommand{
		makeCommand("systemctl", "Controls units",
			makeSubcommand("start", "Start units", makeUnitOption("The unit to start", true, "startable", "restartable")),
			makeSubcommand("stop", "Stop units", makeUnitOption("The unit to stop", true, "stoppable", "restartable")),
			makeSubcommand("restart", "Restart units", makeUnitOption("The unit to restart", true, "restartable")),
			makeSubcommand("status", "Check units' status", makeUnitOption("The unit to check", false, "startable", "stoppable", "restartable")),
		),
	}) {
		t.Error("Not equal")
	}

}

func TestMultipleCommands(t *testing.T) {
	commands, err := getCommands(getBuilderTestCommandUnits(), Multiple)

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}

	if !reflect.DeepEqual(commands, []*discordgo.ApplicationCommand{
		makeCommand("start", "Start units", makeUnitOption("The unit to start", true, "startable", "restartable")),
		makeCommand("stop", "Stop units", makeUnitOption("The unit to stop", true, "stoppable", "restartable")),
		makeCommand("restart", "Restart units", makeUnitOption("The unit to restart", true, "restartable")),
		makeCommand("status", "Check units' status", makeUnitOption("The unit to check", false, "startable", "stoppable", "restartable")),
	}) {
		t.Error("Not equal")
	}
}

func TestInvalidCommandType(t *testing.T) {
	_, err := getCommands(getBuilderTestCommandUnits(), "invalid")

	if err.Error() != "invalid command type" {
		t.Fatalf("Unexpected error %v", err)
	}
}
