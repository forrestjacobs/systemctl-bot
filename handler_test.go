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
	if eq := reflect.DeepEqual(options, expectedOptions); !eq {
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
	if eq := reflect.DeepEqual(options, expectedOptions); !eq {
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
	option       string
}

func (i *mockInteraction) start(unit string) error {
	i.calls = append(i.calls, mockCall{name: "systemd.start", args: []any{unit}})
	return i.systemdError
}

func (i *mockInteraction) stop(unit string) error {
	i.calls = append(i.calls, mockCall{name: "systemd.stop", args: []any{unit}})
	return i.systemdError
}

func (i *mockInteraction) restart(unit string) error {
	i.calls = append(i.calls, mockCall{name: "systemd.restart", args: []any{unit}})
	return i.systemdError
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

func (i *mockInteraction) getOptionStringValue(index int) string {
	return i.option
}

func (i *mockInteraction) respond(content string) error {
	i.calls = append(i.calls, mockCall{name: "respond", args: []any{content}})
	return nil
}

func (i *mockInteraction) deferResponse() error {
	i.calls = append(i.calls, mockCall{name: "deferResponse"})
	return nil
}

func (i *mockInteraction) followUp(content string) error {
	i.calls = append(i.calls, mockCall{name: "followUp", args: []any{content}})
	return nil
}

func TestStartHandler(t *testing.T) {
	i := mockInteraction{
		units:  []string{"startable.service"},
		option: "startable.service",
	}
	commandHandlers[StartCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StartCommand}},
		{name: "deferResponse"},
		{name: "systemd.start", args: []any{"startable.service"}},
		{name: "followUp", args: []any{"Started startable.service"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestStartSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not start"),
		units:        []string{"startable.service"},
		option:       "startable.service",
	}
	commandHandlers[StartCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StartCommand}},
		{name: "deferResponse"},
		{name: "systemd.start", args: []any{"startable.service"}},
		{name: "followUp", args: []any{"could not start"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestStartDisallowedHandler(t *testing.T) {
	i := mockInteraction{
		option: "disallowed.service",
	}
	commandHandlers[StartCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StartCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestStopHandler(t *testing.T) {
	i := mockInteraction{
		units:  []string{"stoppable.service"},
		option: "stoppable.service",
	}
	commandHandlers[StopCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StopCommand}},
		{name: "deferResponse"},
		{name: "systemd.stop", args: []any{"stoppable.service"}},
		{name: "followUp", args: []any{"Stopped stoppable.service"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestStopSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not stop"),
		units:        []string{"stoppable.service"},
		option:       "stoppable.service",
	}
	commandHandlers[StopCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StopCommand}},
		{name: "deferResponse"},
		{name: "systemd.stop", args: []any{"stoppable.service"}},
		{name: "followUp", args: []any{"could not stop"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestStopDisallowedHandler(t *testing.T) {
	i := mockInteraction{
		option: "disallowed.service",
	}
	commandHandlers[StopCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StopCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestRestartHandler(t *testing.T) {
	i := mockInteraction{
		units:  []string{"restartable.service"},
		option: "restartable.service",
	}
	commandHandlers[RestartCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{RestartCommand}},
		{name: "deferResponse"},
		{name: "systemd.restart", args: []any{"restartable.service"}},
		{name: "followUp", args: []any{"Restarted restartable.service"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestRestartSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not restart"),
		units:        []string{"restartable.service"},
		option:       "restartable.service",
	}
	commandHandlers[RestartCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{RestartCommand}},
		{name: "deferResponse"},
		{name: "systemd.restart", args: []any{"restartable.service"}},
		{name: "followUp", args: []any{"could not restart"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestRestartDisallowedHandler(t *testing.T) {
	i := mockInteraction{
		option: "disallowed.service",
	}
	commandHandlers[RestartCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{RestartCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestMultiStatusHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"active.service", "reloading.service", "inactive.service"},
	}
	commandHandlers[StatusCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"active.service"}},
		{name: "systemd.getUnitActiveState", args: []any{"reloading.service"}},
		{name: "systemd.getUnitActiveState", args: []any{"inactive.service"}},
		{name: "respond", args: []any{"active.service: active\nreloading.service: reloading"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestMultiStatusSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not get status"),
		units:        []string{"active.service"},
	}
	commandHandlers[StatusCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"active.service"}},
		{name: "respond", args: []any{"active.service: error getting status"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestNoneActiveStatusHandler(t *testing.T) {
	i := mockInteraction{
		units: []string{"inactive.service"},
	}
	commandHandlers[StatusCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"inactive.service"}},
		{name: "respond", args: []any{"Nothing is active"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestUnitStatusHandler(t *testing.T) {
	i := mockInteraction{
		units:  []string{"reloading.service"},
		option: "reloading.service",
	}
	commandHandlers[StatusCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"reloading.service"}},
		{name: "respond", args: []any{"reloading"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestUnitStatusSystemdErrorHandler(t *testing.T) {
	i := mockInteraction{
		systemdError: errors.New("could not get status"),
		units:        []string{"reloading.service"},
		option:       "reloading.service",
	}
	commandHandlers[StatusCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "systemd.getUnitActiveState", args: []any{"reloading.service"}},
		{name: "respond", args: []any{"could not get status"}},
	}); !eq {
		t.Error("Not equal")
	}
}

func TestDisallowedUnitStatusHandler(t *testing.T) {
	i := mockInteraction{
		option: "disallowed.service",
	}
	commandHandlers[StatusCommand](&i)
	if eq := reflect.DeepEqual(i.calls, []mockCall{
		{name: "getUnits", args: []any{StatusCommand}},
		{name: "respond", args: []any{"command is not allowed"}},
	}); !eq {
		t.Error("Not equal")
	}
}
