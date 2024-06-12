package main

import (
	"errors"
	"reflect"
	"strings"
	"testing"

	"github.com/bwmarrin/discordgo"
)

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

type mockInteraction struct {
	calls        []mockCall
	systemdError error
	units        []string
}

func (i *mockInteraction) makeMockSystemdCallChan() <-chan string {
	c := make(chan string, 1)
	if i.systemdError == nil {
		c <- "done"
	} else {
		c <- "failed"
	}
	return c
}

func (i *mockInteraction) start(unit string) (<-chan string, error) {
	i.calls = append(i.calls, mockCall{name: "systemd.start", args: []any{unit}})
	return i.makeMockSystemdCallChan(), i.systemdError
}

func (i *mockInteraction) stop(unit string) (<-chan string, error) {
	i.calls = append(i.calls, mockCall{name: "systemd.stop", args: []any{unit}})
	return i.makeMockSystemdCallChan(), i.systemdError
}

func (i *mockInteraction) restart(unit string) (<-chan string, error) {
	i.calls = append(i.calls, mockCall{name: "systemd.restart", args: []any{unit}})
	return i.makeMockSystemdCallChan(), i.systemdError
}

func (i *mockInteraction) getUnitActiveState(unit string) (string, error) {
	i.calls = append(i.calls, mockCall{name: "systemd.getUnitActiveState", args: []any{unit}})
	return strings.TrimSuffix(unit, ".service"), i.systemdError
}

func (i *mockInteraction) getSystemd() systemd {
	return i
}

func (i *mockInteraction) getUnits(command command) []string {
	i.calls = append(i.calls, mockCall{name: "getUnits", args: []any{command}})
	return i.units
}

func (i *mockInteraction) respond(content string) {
	i.calls = append(i.calls, mockCall{name: "respond", args: []any{content}})
}

func (i *mockInteraction) deferResponse() bool {
	i.calls = append(i.calls, mockCall{name: "deferResponse"})
	return true
}

func (i *mockInteraction) followUp(content string) {
	i.calls = append(i.calls, mockCall{name: "followUp", args: []any{content}})
}

func makeStringOption(v string) *discordgo.ApplicationCommandInteractionDataOption {
	return &discordgo.ApplicationCommandInteractionDataOption{
		Type:  discordgo.ApplicationCommandOptionString,
		Value: v,
	}
}

func callHandler(command command, i interaction, options ...*discordgo.ApplicationCommandInteractionDataOption) {
	commandHandlers[command](i, options)
}

func TestStartHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"startable.service"},
	}
	callHandler(StartCommand, &i, makeStringOption("startable.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StartCommand}},
		{name: "deferResponse"},
		{name: "systemd.start", args: []any{"startable.service"}},
		{name: "followUp", args: []any{"Started startable.service"}},
	}) {
		t.Error("Not equal")
	}
}

func TestStartSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not start"),
		units:        []string{"startable.service"},
	}
	callHandler(StartCommand, &i, makeStringOption("startable.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StartCommand}},
		{name: "deferResponse"},
		{name: "systemd.start", args: []any{"startable.service"}},
		{name: "followUp", args: []any{"could not start"}},
	}) {
		t.Error("Not equal")
	}
}

func TestStartDisallowedHandler(t *testing.T) {
	i := mockInteraction{}
	callHandler(StartCommand, &i, makeStringOption("disallowed.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StartCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}) {
		t.Error("Not equal")
	}
}

func TestStopHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"stoppable.service"},
	}
	callHandler(StopCommand, &i, makeStringOption("stoppable.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StopCommand}},
		{name: "deferResponse"},
		{name: "systemd.stop", args: []any{"stoppable.service"}},
		{name: "followUp", args: []any{"Stopped stoppable.service"}},
	}) {
		t.Error("Not equal")
	}
}

func TestStopSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not stop"),
		units:        []string{"stoppable.service"},
	}
	callHandler(StopCommand, &i, makeStringOption("stoppable.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StopCommand}},
		{name: "deferResponse"},
		{name: "systemd.stop", args: []any{"stoppable.service"}},
		{name: "followUp", args: []any{"could not stop"}},
	}) {
		t.Error("Not equal")
	}
}

func TestStopDisallowedHandler(t *testing.T) {
	i := mockInteraction{}
	callHandler(StopCommand, &i, makeStringOption("disallowed.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StopCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}) {
		t.Error("Not equal")
	}
}

func TestRestartHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"restartable.service"},
	}
	callHandler(RestartCommand, &i, makeStringOption("restartable.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{RestartCommand}},
		{name: "deferResponse"},
		{name: "systemd.restart", args: []any{"restartable.service"}},
		{name: "followUp", args: []any{"Restarted restartable.service"}},
	}) {
		t.Error("Not equal")
	}
}

func TestRestartSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not restart"),
		units:        []string{"restartable.service"},
	}
	callHandler(RestartCommand, &i, makeStringOption("restartable.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{RestartCommand}},
		{name: "deferResponse"},
		{name: "systemd.restart", args: []any{"restartable.service"}},
		{name: "followUp", args: []any{"could not restart"}},
	}) {
		t.Error("Not equal")
	}
}

func TestRestartDisallowedHandler(t *testing.T) {
	i := mockInteraction{}
	callHandler(RestartCommand, &i, makeStringOption("disallowed.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{RestartCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}) {
		t.Error("Not equal")
	}
}

func TestMultiStatusHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"active.service", "reloading.service", "inactive.service"},
	}
	callHandler(StatusCommand, &i)
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"active.service"}},
		{name: "systemd.getUnitActiveState", args: []any{"reloading.service"}},
		{name: "systemd.getUnitActiveState", args: []any{"inactive.service"}},
		{name: "respond", args: []any{"active.service: active\nreloading.service: reloading"}},
	}) {
		t.Error("Not equal")
	}
}

func TestMultiStatusSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not get status"),
		units:        []string{"active.service"},
	}
	callHandler(StatusCommand, &i)
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"active.service"}},
		{name: "respond", args: []any{"active.service: error getting status"}},
	}) {
		t.Error("Not equal")
	}
}

func TestNoneActiveStatusHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"inactive.service"},
	}
	callHandler(StatusCommand, &i)
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"inactive.service"}},
		{name: "respond", args: []any{"Nothing is active"}},
	}) {
		t.Error("Not equal")
	}
}

func TestUnitStatusHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"reloading.service"},
	}
	callHandler(StatusCommand, &i, makeStringOption("reloading.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"reloading.service"}},
		{name: "respond", args: []any{"reloading"}},
	}) {
		t.Error("Not equal")
	}
}

func TestUnitStatusSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not get status"),
		units:        []string{"reloading.service"},
	}
	callHandler(StatusCommand, &i, makeStringOption("reloading.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"reloading.service"}},
		{name: "respond", args: []any{"could not get status"}},
	}) {
		t.Error("Not equal")
	}
}

func TestDisallowedUnitStatusHandler(t *testing.T) {
	i := mockInteraction{}
	callHandler(StatusCommand, &i, makeStringOption("disallowed.service"))
	if !reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}) {
		t.Error("Not equal")
	}
}
