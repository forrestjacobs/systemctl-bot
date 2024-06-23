package main

import (
	"context"

	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/samber/lo"
)

type systemd interface {
	StartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	StopUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	RestartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	GetUnitPropertyContext(ctx context.Context, unit string, propertyName string) (*dbus.Property, error)
}

type systemdSubscriptionSet interface {
	Add(value string)
	Subscribe() (<-chan map[string]*dbus.UnitStatus, <-chan error)
}

func subscribeToActiveUnits(subscription systemdSubscriptionSet, units []string) (<-chan []string, <-chan error) {
	for _, unit := range units {
		subscription.Add(unit)
	}

	statusChan, errChan := subscription.Subscribe()

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
	return activeChan, errChan
}
