package main

import (
	"context"
	"os/exec"

	"github.com/coreos/go-systemd/v22/dbus"
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

func (s *systemdImpl) subscribeToActiveUnits(units []string) (<-chan map[string]bool, <-chan error) {
	subscription := s.conn.NewSubscriptionSet()
	for _, unit := range units {
		subscription.Add(unit)
	}

	activeStatesChan := make(chan map[string]bool)
	statusChan, errChan := subscription.Subscribe()

	go func() {
		activeStates := make(map[string]bool)
		for statuses := range statusChan {
			for name, status := range statuses {
				activeStates[name] = status.ActiveState == "active"
			}
			activeStatesChan <- activeStates
		}
	}()

	return activeStatesChan, errChan
}
