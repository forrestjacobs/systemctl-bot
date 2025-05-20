package handler_test

import (
	"context"
	"errors"
	"reflect"
	"testing"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
	"github.com/forrestjacobs/systemctl-bot/internal/handler"
)

var baseConfig = &config.Config{
	CommandUnits: map[config.Command][]string{
		config.StartCommand:   {"startable.service"},
		config.StopCommand:    {"stoppable.service"},
		config.RestartCommand: {"restartable.service"},
	},
}

func makeStringOption(v string) *discordgo.ApplicationCommandInteractionDataOption {
	return &discordgo.ApplicationCommandInteractionDataOption{
		Type:  discordgo.ApplicationCommandOptionString,
		Value: v,
	}
}

type mockCall struct {
	name string
	args []any
}

type handlerMocks struct {
	calls         []mockCall
	handler       interface{}
	discordError  error
	systemdResult string
	systemdError  error
}

func (s *handlerMocks) AddHandler(handler interface{}) func() {
	s.handler = handler
	return func() {}
}

func (s *handlerMocks) InteractionRespond(interaction *discordgo.Interaction, resp *discordgo.InteractionResponse, options ...discordgo.RequestOption) error {
	if len(options) > 0 {
		panic("Cannot handle options")
	}
	s.calls = append(s.calls, mockCall{name: "session.InteractionRespond", args: []any{interaction, resp}})
	return s.discordError
}

func (s *handlerMocks) FollowupMessageCreate(interaction *discordgo.Interaction, wait bool, data *discordgo.WebhookParams, options ...discordgo.RequestOption) (*discordgo.Message, error) {
	if len(options) > 0 {
		panic("Cannot handle options")
	}
	s.calls = append(s.calls, mockCall{name: "session.FollowupMessageCreate", args: []any{interaction, wait, data}})
	return nil, s.discordError
}

func (s *handlerMocks) handleUnitCommand(funcName string, unitName string, mode string, ch chan<- string) (int, error) {
	s.calls = append(s.calls, mockCall{name: "systemd." + funcName, args: []any{unitName, mode}})
	go func() {
		if s.systemdError != nil {
			ch <- "failed"
		} else if s.systemdResult != "" {
			ch <- s.systemdResult
		} else {
			ch <- "done"
		}
	}()
	return 0, s.systemdError
}

func (s *handlerMocks) StartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error) {
	return s.handleUnitCommand("StartUnitContext", name, mode, ch)
}

func (s *handlerMocks) StopUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error) {
	return s.handleUnitCommand("StopUnitContext", name, mode, ch)
}

func (s *handlerMocks) RestartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error) {
	return s.handleUnitCommand("RestartUnitContext", name, mode, ch)
}

func makeSystemdInteraction(command config.Command, options ...*discordgo.ApplicationCommandInteractionDataOption) *discordgo.Interaction {
	return &discordgo.Interaction{
		Type: discordgo.InteractionApplicationCommand,
		Data: discordgo.ApplicationCommandInteractionData{
			Name: "systemctl",
			Options: []*discordgo.ApplicationCommandInteractionDataOption{
				{
					Name:    string(command),
					Options: options,
				},
			},
		},
	}
}

func callHandler(mocks *handlerMocks, config *config.Config, interaction *discordgo.Interaction) {
	handler.AddHandler(mocks, mocks, config)
	mocks.handler.(func(session *discordgo.Session, event *discordgo.InteractionCreate))(nil, &discordgo.InteractionCreate{
		Interaction: interaction,
	})
}

func mockCallForDeferResponse(interaction *discordgo.Interaction) mockCall {
	resp := discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	}
	return mockCall{
		name: "session.InteractionRespond",
		args: []any{interaction, &resp},
	}
}

func mockCallForFollowUp(interaction *discordgo.Interaction, content string) mockCall {
	data := discordgo.WebhookParams{Content: content}
	return mockCall{
		name: "session.FollowupMessageCreate",
		args: []any{interaction, false, &data},
	}
}

func mockCallForUnitAction(verb string, unit string) mockCall {
	return mockCall{
		name: "systemd." + verb + "UnitContext",
		args: []any{unit, "replace"},
	}
}

func TestOnlyHandleApplicationCommands(t *testing.T) {
	m := handlerMocks{}
	interaction := &discordgo.Interaction{
		Type: discordgo.InteractionPing,
	}
	callHandler(&m, baseConfig, interaction)
	if len(m.calls) > 0 {
		t.Error("Unexpected calls")
	}
}

func TestStartHandlerWithSystemdSlashCommand(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StartCommand, makeStringOption("startable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Start", "startable.service"),
		mockCallForFollowUp(interaction, "Started startable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestStartHandlerWithStartSlashCommand(t *testing.T) {
	m := handlerMocks{}

	interaction := &discordgo.Interaction{
		Type: discordgo.InteractionApplicationCommand,
		Data: discordgo.ApplicationCommandInteractionData{
			Name: "start",
			Options: []*discordgo.ApplicationCommandInteractionDataOption{
				makeStringOption("startable.service"),
			},
		},
	}

	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Start", "startable.service"),
		mockCallForFollowUp(interaction, "Started startable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestStartSystemdTimeoutHandler(t *testing.T) {
	m := handlerMocks{systemdResult: "timeout"}
	interaction := makeSystemdInteraction(config.StartCommand, makeStringOption("startable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Start", "startable.service"),
		mockCallForFollowUp(interaction, "timeout"),
	}) {
		t.Error("Not equal")
	}
}

func TestStartDiscordErrorHandler(t *testing.T) {
	m := handlerMocks{discordError: errors.New("discord error")}
	interaction := makeSystemdInteraction(config.StartCommand, makeStringOption("startable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
	}) {
		t.Error("Not equal")
	}
}

func TestStartSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not start")}
	interaction := makeSystemdInteraction(config.StartCommand, makeStringOption("startable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Start", "startable.service"),
		mockCallForFollowUp(interaction, "could not start"),
	}) {
		t.Error("Not equal")
	}
}

func TestStartDisallowedHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StartCommand, makeStringOption("disallowed.service"))
	callHandler(&m, baseConfig, interaction)
	if len(m.calls) > 0 {
		t.Error("Unexpected calls")
	}
}

func TestStopHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StopCommand, makeStringOption("stoppable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Stop", "stoppable.service"),
		mockCallForFollowUp(interaction, "Stopped stoppable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestStopTimeoutHandler(t *testing.T) {
	m := handlerMocks{systemdResult: "timeout"}
	interaction := makeSystemdInteraction(config.StopCommand, makeStringOption("stoppable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Stop", "stoppable.service"),
		mockCallForFollowUp(interaction, "timeout"),
	}) {
		t.Error("Not equal")
	}
}

func TestStopSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not stop")}
	interaction := makeSystemdInteraction(config.StopCommand, makeStringOption("stoppable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Stop", "stoppable.service"),
		mockCallForFollowUp(interaction, "could not stop"),
	}) {
		t.Error("Not equal")
	}
}

func TestStopDisallowedHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StopCommand, makeStringOption("disallowed.service"))
	callHandler(&m, baseConfig, interaction)
	if len(m.calls) > 0 {
		t.Error("Unexpected calls")
	}
}

func TestRestartHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.RestartCommand, makeStringOption("restartable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Restart", "restartable.service"),
		mockCallForFollowUp(interaction, "Restarted restartable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestRestartTimeoutHandler(t *testing.T) {
	m := handlerMocks{systemdResult: "timeout"}
	interaction := makeSystemdInteraction(config.RestartCommand, makeStringOption("restartable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Restart", "restartable.service"),
		mockCallForFollowUp(interaction, "timeout"),
	}) {
		t.Error("Not equal")
	}
}

func TestRestartSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not restart")}
	interaction := makeSystemdInteraction(config.RestartCommand, makeStringOption("restartable.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(interaction),
		mockCallForUnitAction("Restart", "restartable.service"),
		mockCallForFollowUp(interaction, "could not restart"),
	}) {
		t.Error("Not equal")
	}
}

func TestRestartDisallowedHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.RestartCommand, makeStringOption("disallowed.service"))
	callHandler(&m, baseConfig, interaction)
	if len(m.calls) > 0 {
		t.Error("Unexpected calls")
	}
}
