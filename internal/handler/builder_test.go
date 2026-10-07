package handler_test

import (
	"reflect"
	"testing"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
	"github.com/forrestjacobs/systemctl-bot/internal/handler"
)

type mockDiscordSession struct {
	calls [][]any
}

func (s *mockDiscordSession) ApplicationCommandBulkOverwrite(appID string, guildID string, commands []*discordgo.ApplicationCommand, options ...discordgo.RequestOption) (createdCommands []*discordgo.ApplicationCommand, err error) {
	s.calls = append(s.calls, []any{
		"ApplicationCommandBulkOverwrite", appID, guildID, commands,
	})
	return commands, nil
}

func getBuilderTestUnits() []*config.UnitConfig {
	return []*config.UnitConfig{
		{
			Name:        "startable.service",
			Permissions: []config.Permission{config.StartPermission},
		},
		{
			Name:        "stoppable.service",
			Permissions: []config.Permission{config.StopPermission},
		},
		{
			Name:        "restartable.service",
			Permissions: []config.Permission{config.StartPermission, config.StopPermission},
		},
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
	choices := []*discordgo.ApplicationCommandOptionChoice{}
	for _, name := range units {
		choices = append(choices, &discordgo.ApplicationCommandOptionChoice{
			Name:  name,
			Value: name + ".service",
		})
	}

	return &discordgo.ApplicationCommandOption{
		Name:        "unit",
		Type:        discordgo.ApplicationCommandOptionString,
		Description: description,
		Required:    required,
		Choices:     choices,
	}
}

func TestGetSingleCommand(t *testing.T) {
	s := &mockDiscordSession{}
	c := &config.Config{
		ApplicationID: 1,
		GuildID:       2,
		CommandType:   config.Single,
		Units:         getBuilderTestUnits(),
	}
	err := handler.RegisterCommands(s, c)

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}

	if !reflect.DeepEqual(s.calls, [][]any{{
		"ApplicationCommandBulkOverwrite", "1", "2",
		[]*discordgo.ApplicationCommand{
			makeCommand("systemctl", "Controls units",
				makeSubcommand("start", "Start units", makeUnitOption("The unit to start", true, "startable", "restartable")),
				makeSubcommand("stop", "Stop units", makeUnitOption("The unit to stop", true, "stoppable", "restartable")),
				makeSubcommand("restart", "Restart units", makeUnitOption("The unit to restart", true, "restartable")),
			),
		},
	},
	}) {
		t.Error("Not equal")
	}

}

func TestMultipleCommands(t *testing.T) {
	s := &mockDiscordSession{}
	c := &config.Config{
		ApplicationID: 1,
		GuildID:       2,
		CommandType:   config.Multiple,
		Units:         getBuilderTestUnits(),
	}
	err := handler.RegisterCommands(s, c)

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}

	if !reflect.DeepEqual(s.calls, [][]any{{
		"ApplicationCommandBulkOverwrite", "1", "2",
		[]*discordgo.ApplicationCommand{
			makeCommand("start", "Start units", makeUnitOption("The unit to start", true, "startable", "restartable")),
			makeCommand("stop", "Stop units", makeUnitOption("The unit to stop", true, "stoppable", "restartable")),
			makeCommand("restart", "Restart units", makeUnitOption("The unit to restart", true, "restartable")),
		},
	}}) {
		t.Error("Not equal")
	}
}

func TestInvalidCommandType(t *testing.T) {
	s := &mockDiscordSession{}
	c := &config.Config{
		ApplicationID: 1,
		GuildID:       2,
		CommandType:   "invalid",
		Units:         getBuilderTestUnits(),
	}
	err := handler.RegisterCommands(s, c)

	if err.Error() != "invalid command type" {
		t.Fatalf("Unexpected error %v", err)
	}
}
