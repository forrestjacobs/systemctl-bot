package main

import (
	"reflect"
	"testing"
)

func TestGetCommandUnits(t *testing.T) {
	commandUnits := getCommandUnits([]*systemctlUnit{
		{
			Name:        "none.service",
			Permissions: []unitPermission{},
		},
		{
			Name:        "startable.service",
			Permissions: []unitPermission{StartPermission, StatusPermission},
		},
		{
			Name:        "stoppable.service",
			Permissions: []unitPermission{StopPermission, StatusPermission},
		},
		{
			Name:        "restartable.service",
			Permissions: []unitPermission{StartPermission, StopPermission, StatusPermission},
		},
	})
	if !reflect.DeepEqual(commandUnits, map[command][]string{
		StartCommand:   {"startable.service", "restartable.service"},
		StopCommand:    {"stoppable.service", "restartable.service"},
		RestartCommand: {"restartable.service"},
		StatusCommand:  {"startable.service", "stoppable.service", "restartable.service"},
	}) {
		t.Error("Not equal")
	}
}
