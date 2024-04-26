package main

import (
	"context"
	"os/exec"

	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/samber/lo"
)

type systemd interface {
	start(unit string) error
	stop(unit string) error
	restart(unit string) error
	getUnitActiveState(unit string) (string, error)
}

type systemdImpl struct {
	conn *dbus.Conn
}

func (s *systemdImpl) start(unit string) error {
	return exec.Command("systemctl", "start", unit).Run()
}

func (s *systemdImpl) stop(unit string) error {
	return exec.Command("systemctl", "stop", unit).Run()
}

func (s *systemdImpl) restart(unit string) error {
	return exec.Command("systemctl", "restart", unit).Run()
}

func (s *systemdImpl) getUnitActiveState(unit string) (string, error) {
	prop, err := s.conn.GetUnitPropertyContext(context.Background(), unit, "ActiveState")
	if err != nil {
		return "", err
	}
	return prop.Value.Value().(string), nil
}

func subscribeToUnits(conn *dbus.Conn, units []string) *dbus.SubscriptionSet {
	subscription := conn.NewSubscriptionSet()
	for _, unit := range units {
		subscription.Add(unit)
	}
	return subscription
}

func transformStatusChanToActiveList(units []string, statusChan <-chan map[string]*dbus.UnitStatus) <-chan []string {
	activeChan := make(chan []string)
	go func() {
		activeStates := make(map[string]bool)
		for statuses := range statusChan {
			for name, status := range statuses {
				activeStates[name] = status.ActiveState == "active"
			}
			activeList := lo.FilterMap(units, func(unit string, _ int) (string, bool) {
				return unit, activeStates[unit]
			})
			activeChan <- activeList
		}
	}()
	return activeChan
}

func (s *systemdImpl) subscribeToActiveUnits(units []string) (<-chan []string, <-chan error) {
	statusChan, errChan := subscribeToUnits(s.conn, units).Subscribe()
	return transformStatusChanToActiveList(units, statusChan), errChan
}
