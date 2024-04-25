package main

import "github.com/samber/lo"

type unitPermission string

const (
	StartPermission  unitPermission = "start"
	StopPermission   unitPermission = "stop"
	StatusPermission unitPermission = "status"
)

type command string

const (
	StartCommand   command = "start"
	StopCommand    command = "stop"
	RestartCommand command = "restart"
	StatusCommand  command = "status"
)

type systemctlUnit struct {
	Name        string
	Permissions []unitPermission
}

func getUnitsWithPermissions(units []*systemctlUnit, permissions ...unitPermission) []string {
	return lo.FilterMap(units, func(unit *systemctlUnit, _ int) (string, bool) {
		return unit.Name, lo.Every(unit.Permissions, permissions)
	})
}

func getCommandUnits(units []*systemctlUnit) map[command][]string {
	return map[command][]string{
		StartCommand:   getUnitsWithPermissions(units, StartPermission),
		StopCommand:    getUnitsWithPermissions(units, StopPermission),
		RestartCommand: getUnitsWithPermissions(units, StartPermission, StopPermission),
		StatusCommand:  getUnitsWithPermissions(units, StatusPermission),
	}
}
