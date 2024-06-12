package main

import (
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/samber/lo"
)

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

func subscribeToActiveUnits(conn *dbus.Conn, units []string) (<-chan []string, <-chan error) {
	statusChan, errChan := subscribeToUnits(conn, units).Subscribe()
	return transformStatusChanToActiveList(units, statusChan), errChan
}
