package main

import (
	"reflect"
	"testing"

	"github.com/bwmarrin/discordgo"
)

type testDiscordSession struct {
}

func (d *testDiscordSession) InteractionRespond(interaction *discordgo.Interaction, resp *discordgo.InteractionResponse, options ...discordgo.RequestOption) error {
	return nil
}
func (d *testDiscordSession) FollowupMessageCreate(interaction *discordgo.Interaction, wait bool, data *discordgo.WebhookParams, options ...discordgo.RequestOption) (*discordgo.Message, error) {
	return nil, nil
}

type testCommandRunner struct {
	calls []mockCall
}

func (runner *testCommandRunner) run(ctx *commandCtx) {
	runner.calls = append(runner.calls, mockCall{
		name: "run",
		args: []any{ctx},
	})
}

func TestOnlyHandleApplicationCommands(t *testing.T) {
	session := &testDiscordSession{}
	runner := &testCommandRunner{}
	handler := makeInteractionHandler(runner)
	handler(session, &discordgo.InteractionCreate{
		Interaction: &discordgo.Interaction{
			Type: discordgo.InteractionPing,
		},
	})
	if len(runner.calls) > 0 {
		t.Error("Unexpected calls")
	}
}

func TestHandleSingleCommandData(t *testing.T) {
	session := &testDiscordSession{}
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
	handler(session, &discordgo.InteractionCreate{
		Interaction: interaction,
	})

	if !reflect.DeepEqual(runner.calls, []mockCall{
		{
			name: "run",
			args: []any{&commandCtx{
				commandName: "start",
				options:     options,
				session:     session,
				interaction: interaction,
			}},
		},
	}) {
		t.Error("Not equal")
	}
}

func TestHandleMultipleCommandData(t *testing.T) {
	session := &testDiscordSession{}
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
	handler(session, &discordgo.InteractionCreate{
		Interaction: interaction,
	})

	if !reflect.DeepEqual(runner.calls, []mockCall{
		{
			name: "run",
			args: []any{&commandCtx{
				commandName: "stop",
				options:     options,
				session:     session,
				interaction: interaction,
			}},
		},
	}) {
		t.Error("Not equal")
	}
}
