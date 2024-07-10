package handler_test

import (
	"context"
	"errors"
	"reflect"
	"strings"
	"testing"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
	"github.com/forrestjacobs/systemctl-bot/internal/handler"
	godbus "github.com/godbus/dbus/v5"
)

var baseConfig = &config.Config{
	Units: map[config.Command][]string{
		config.StartCommand:   {"startable.service"},
		config.StopCommand:    {"stoppable.service"},
		config.RestartCommand: {"restartable.service"},
		config.StatusCommand:  {"active.service", "reloading.service", "inactive.service"},
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

func (s *handlerMocks) GetUnitPropertyContext(ctx context.Context, unit string, propertyName string) (*dbus.Property, error) {
	s.calls = append(s.calls, mockCall{name: "systemd.GetUnitPropertyContext", args: []any{unit, propertyName}})
	if s.systemdError != nil {
		return nil, s.systemdError
	}
	return &dbus.Property{
		Name:  propertyName,
		Value: godbus.MakeVariant(strings.TrimSuffix(unit, ".service")),
	}, nil
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
	mocks.handler.(func(session handler.DiscordSession, event *discordgo.InteractionCreate))(mocks, &discordgo.InteractionCreate{
		Interaction: interaction,
	})
}

func mockCallForRespond(interaction *discordgo.Interaction, content string) mockCall {
	resp := discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
		},
	}
	return mockCall{
		name: "session.InteractionRespond",
		args: []any{interaction, &resp},
	}
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

func mockCallForGetUnitActiveState(unit string) mockCall {
	return mockCall{
		name: "systemd.GetUnitPropertyContext",
		args: []any{unit, "ActiveState"},
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

func TestMultiStatusHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StatusCommand)
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("active.service"),
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForGetUnitActiveState("inactive.service"),
		mockCallForRespond(interaction, "active.service: active\nreloading.service: reloading"),
	}) {
		t.Error("Not equal")
	}
}

func TestMultiStatusSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{
		systemdError: errors.New("could not get status"),
	}
	interaction := makeSystemdInteraction(config.StatusCommand)
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("active.service"),
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForGetUnitActiveState("inactive.service"),
		mockCallForRespond(interaction, "active.service: error getting status\nreloading.service: error getting status\ninactive.service: error getting status"),
	}) {
		t.Error("Not equal")
	}
}

func TestNoneActiveStatusHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StatusCommand)
	callHandler(&m, &config.Config{
		Units: map[config.Command][]string{
			config.StatusCommand: {"inactive.service"},
		},
	}, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("inactive.service"),
		mockCallForRespond(interaction, "Nothing is active"),
	}) {
		t.Error("Not equal")
	}
}

func TestUnitStatusHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StatusCommand, makeStringOption("reloading.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForRespond(interaction, "reloading"),
	}) {
		t.Error("Not equal")
	}
}

func TestUnitStatusSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not get status")}
	interaction := makeSystemdInteraction(config.StatusCommand, makeStringOption("reloading.service"))
	callHandler(&m, baseConfig, interaction)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForRespond(interaction, "could not get status"),
	}) {
		t.Error("Not equal")
	}
}

func TestDisallowedUnitStatusHandler(t *testing.T) {
	m := handlerMocks{}
	interaction := makeSystemdInteraction(config.StatusCommand, makeStringOption("disallowed.service"))
	callHandler(&m, baseConfig, interaction)
	if len(m.calls) > 0 {
		t.Error("Unexpected calls")
	}
}
