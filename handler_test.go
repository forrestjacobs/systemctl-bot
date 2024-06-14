package main

import (
	"context"
	"errors"
	"reflect"
	"strings"
	"testing"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	godbus "github.com/godbus/dbus/v5"
)

var testInteraction = &discordgo.Interaction{ID: "12345"}

func TestGetSingleCommandData(t *testing.T) {
	expectedName := "start"
	expectedOptions := []*discordgo.ApplicationCommandInteractionDataOption{
		{
			Type:  discordgo.ApplicationCommandOptionString,
			Value: "startable.service",
		},
	}
	data := discordgo.ApplicationCommandInteractionData{
		Name: "systemctl",
		Options: []*discordgo.ApplicationCommandInteractionDataOption{
			{
				Name:    expectedName,
				Options: expectedOptions,
			},
		},
	}
	name, options := getCommandData(&data)
	if name != expectedName {
		t.Fatalf("Unexpected name %v", name)
	}
	if !reflect.DeepEqual(options, expectedOptions) {
		t.Error("Not equal")
	}
}

func TestGetMultipleCommandData(t *testing.T) {
	expectedName := "stop"
	expectedOptions := []*discordgo.ApplicationCommandInteractionDataOption{
		{
			Type:  discordgo.ApplicationCommandOptionString,
			Value: "stoppable.service",
		},
	}
	data := discordgo.ApplicationCommandInteractionData{
		Name:    expectedName,
		Options: expectedOptions,
	}
	name, options := getCommandData(&data)
	if name != expectedName {
		t.Fatalf("Unexpected name %v", name)
	}
	if !reflect.DeepEqual(options, expectedOptions) {
		t.Error("Not equal")
	}
}

type mockCall struct {
	name string
	args []any
}

type handlerMocks struct {
	calls        []mockCall
	discordError error
	systemdError error
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
		if s.systemdError == nil {
			ch <- "done"
		} else {
			ch <- "failed"
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

func makeStringOption(v string) *discordgo.ApplicationCommandInteractionDataOption {
	return &discordgo.ApplicationCommandInteractionDataOption{
		Type:  discordgo.ApplicationCommandOptionString,
		Value: v,
	}
}

func makeCtx(mocks *handlerMocks) *handlerCtx {
	return &handlerCtx{
		systemd:     mocks,
		session:     mocks,
		interaction: testInteraction,
		commandUnits: map[command][]string{
			StartCommand:   {"startable.service"},
			StopCommand:    {"stoppable.service"},
			RestartCommand: {"restartable.service"},
			StatusCommand:  {"active.service", "reloading.service", "inactive.service"},
		},
	}
}

func callHandler(cmd command, ctx *handlerCtx, options ...*discordgo.ApplicationCommandInteractionDataOption) {
	commandHandlers[cmd](ctx, options)
}

func mockCallForRespond(content string) mockCall {
	resp := discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
		},
	}
	return mockCall{
		name: "session.InteractionRespond",
		args: []any{testInteraction, &resp},
	}
}

func mockCallForDeferResponse() mockCall {
	resp := discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	}
	return mockCall{
		name: "session.InteractionRespond",
		args: []any{testInteraction, &resp},
	}
}

func mockCallForFollowUp(content string) mockCall {
	data := discordgo.WebhookParams{Content: content}
	return mockCall{
		name: "session.FollowupMessageCreate",
		args: []any{testInteraction, false, &data},
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

func TestStartHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StartCommand, makeCtx(&m), makeStringOption("startable.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(),
		mockCallForUnitAction("Start", "startable.service"),
		mockCallForFollowUp("Started startable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestStartSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not start")}
	callHandler(StartCommand, makeCtx(&m), makeStringOption("startable.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(),
		mockCallForUnitAction("Start", "startable.service"),
		mockCallForFollowUp("could not start"),
	}) {
		t.Error("Not equal")
	}
}

func TestStartDisallowedHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StartCommand, makeCtx(&m), makeStringOption("disallowed.service"))
	if len(m.calls) > 0 {
		t.Error("Not equal")
	}
}

func TestStopHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StopCommand, makeCtx(&m), makeStringOption("stoppable.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(),
		mockCallForUnitAction("Stop", "stoppable.service"),
		mockCallForFollowUp("Stopped stoppable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestStopSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not stop")}
	callHandler(StopCommand, makeCtx(&m), makeStringOption("stoppable.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(),
		mockCallForUnitAction("Stop", "stoppable.service"),
		mockCallForFollowUp("could not stop"),
	}) {
		t.Error("Not equal")
	}
}

func TestStopDisallowedHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StopCommand, makeCtx(&m), makeStringOption("disallowed.service"))
	if len(m.calls) > 0 {
		t.Error("Not equal")
	}
}

func TestRestartHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(RestartCommand, makeCtx(&m), makeStringOption("restartable.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(),
		mockCallForUnitAction("Restart", "restartable.service"),
		mockCallForFollowUp("Restarted restartable.service"),
	}) {
		t.Error("Not equal")
	}
}

func TestRestartSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not restart")}
	callHandler(RestartCommand, makeCtx(&m), makeStringOption("restartable.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForDeferResponse(),
		mockCallForUnitAction("Restart", "restartable.service"),
		mockCallForFollowUp("could not restart"),
	}) {
		t.Error("Not equal")
	}
}

func TestRestartDisallowedHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(RestartCommand, makeCtx(&m), makeStringOption("disallowed.service"))
	if len(m.calls) > 0 {
		t.Error("Not equal")
	}
}

func TestMultiStatusHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StatusCommand, makeCtx(&m))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("active.service"),
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForGetUnitActiveState("inactive.service"),
		mockCallForRespond("active.service: active\nreloading.service: reloading"),
	}) {
		t.Error("Not equal")
	}
}

func TestMultiStatusSystemdErrorHandler(t *testing.T) {
	i := handlerMocks{
		systemdError: errors.New("could not get status"),
	}
	callHandler(StatusCommand, makeCtx(&i))
	if !reflect.DeepEqual(i.calls, []mockCall{
		mockCallForGetUnitActiveState("active.service"),
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForGetUnitActiveState("inactive.service"),
		mockCallForRespond("active.service: error getting status\nreloading.service: error getting status\ninactive.service: error getting status"),
	}) {
		t.Error("Not equal")
	}
}

func TestNoneActiveStatusHandler(t *testing.T) {
	m := handlerMocks{}
	ctx := makeCtx(&m)
	ctx.commandUnits[StatusCommand] = []string{"inactive.service"}
	callHandler(StatusCommand, ctx)
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("inactive.service"),
		mockCallForRespond("Nothing is active"),
	}) {
		t.Error("Not equal")
	}
}

func TestUnitStatusHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StatusCommand, makeCtx(&m), makeStringOption("reloading.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForRespond("reloading"),
	}) {
		t.Error("Not equal")
	}
}

func TestUnitStatusSystemdErrorHandler(t *testing.T) {
	m := handlerMocks{systemdError: errors.New("could not get status")}
	callHandler(StatusCommand, makeCtx(&m), makeStringOption("reloading.service"))
	if !reflect.DeepEqual(m.calls, []mockCall{
		mockCallForGetUnitActiveState("reloading.service"),
		mockCallForRespond("could not get status"),
	}) {
		t.Error("Not equal")
	}
}

func TestDisallowedUnitStatusHandler(t *testing.T) {
	m := handlerMocks{}
	callHandler(StatusCommand, makeCtx(&m), makeStringOption("disallowed.service"))
	if len(m.calls) > 0 {
		t.Error("Not equal")
	}
}
