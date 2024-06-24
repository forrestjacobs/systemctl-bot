package handler

import (
	"reflect"
	"testing"

	"github.com/bwmarrin/discordgo"
)

func makeStringOption(v string) *discordgo.ApplicationCommandInteractionDataOption {
	return &discordgo.ApplicationCommandInteractionDataOption{
		Type:  discordgo.ApplicationCommandOptionString,
		Value: v,
	}
}

type testCommandRunner struct {
	calls [][]any
}

func (runner *testCommandRunner) run(ctx *commandCtx) {
	runner.calls = append(runner.calls, []any{"run", ctx})
}

func TestOnlyHandleApplicationCommands(t *testing.T) {
	runner := &testCommandRunner{}
	handler := makeInteractionHandler(runner)
	handler(nil, &discordgo.InteractionCreate{
		Interaction: &discordgo.Interaction{
			Type: discordgo.InteractionPing,
		},
	})
	if len(runner.calls) > 0 {
		t.Error("Unexpected calls")
	}
}

func TestHandleSingleCommandData(t *testing.T) {
	options := []*discordgo.ApplicationCommandInteractionDataOption{
		makeStringOption("startable.service"),
	}
	interaction := &discordgo.Interaction{
		Type: discordgo.InteractionApplicationCommand,
		Data: discordgo.ApplicationCommandInteractionData{
			Name: "systemctl",
			Options: []*discordgo.ApplicationCommandInteractionDataOption{
				{
					Name:    "start",
					Options: options,
				},
			},
		},
	}

	runner := &testCommandRunner{}
	handler := makeInteractionHandler(runner)
	handler(nil, &discordgo.InteractionCreate{
		Interaction: interaction,
	})

	if !reflect.DeepEqual(runner.calls, [][]any{
		{
			"run",
			&commandCtx{
				commandName: "start",
				options:     options,
				session:     nil,
				interaction: interaction,
			},
		},
	}) {
		t.Error("Not equal")
	}
}

func TestHandleMultipleCommandData(t *testing.T) {
	options := []*discordgo.ApplicationCommandInteractionDataOption{
		makeStringOption("stoppable.service"),
	}
	interaction := &discordgo.Interaction{
		Type: discordgo.InteractionApplicationCommand,
		Data: discordgo.ApplicationCommandInteractionData{
			Name:    "stop",
			Options: options,
		},
	}

	runner := &testCommandRunner{}
	handler := makeInteractionHandler(runner)
	handler(nil, &discordgo.InteractionCreate{
		Interaction: interaction,
	})

	if !reflect.DeepEqual(runner.calls, [][]any{
		{
			"run",
			&commandCtx{
				commandName: "stop",
				options:     options,
				session:     nil,
				interaction: interaction,
			},
		},
	}) {
		t.Error("Not equal")
	}
}
