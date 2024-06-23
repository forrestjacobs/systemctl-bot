package main

import (
	"reflect"
	"testing"

	"github.com/coreos/go-systemd/v22/dbus"
)

type MockSubscriptionSet struct {
	mockCalls  []mockCall
	statusChan chan map[string]*dbus.UnitStatus
	errChan    chan error
}

func (m *MockSubscriptionSet) Add(value string) {
	m.mockCalls = append(m.mockCalls, mockCall{
		name: "Add",
		args: []any{value},
	})
}

func (m *MockSubscriptionSet) Subscribe() (<-chan map[string]*dbus.UnitStatus, <-chan error) {
	m.mockCalls = append(m.mockCalls, mockCall{name: "Subscribe"})
	return m.statusChan, m.errChan
}

func TestSubscribeToActiveUnits(t *testing.T) {
	s := MockSubscriptionSet{
		statusChan: make(chan map[string]*dbus.UnitStatus),
	}

	activeChan, _ := subscribeToActiveUnits(&s, []string{"a.service", "b.service"})

	if !reflect.DeepEqual(s.mockCalls, []mockCall{
		{name: "Add", args: []any{"a.service"}},
		{name: "Add", args: []any{"b.service"}},
		{name: "Subscribe"},
	}) {
		t.Error("Not equal")
	}

	go func() {
		s.statusChan <- map[string]*dbus.UnitStatus{
			"a.service": {ActiveState: "active"},
		}
	}()
	if !reflect.DeepEqual(<-activeChan, []string{"a.service"}) {
		t.Error("Not equal")
	}

	go func() {
		s.statusChan <- map[string]*dbus.UnitStatus{
			"b.service": {ActiveState: "active"},
		}
	}()
	if !reflect.DeepEqual(<-activeChan, []string{"a.service", "b.service"}) {
		t.Error("Not equal")
	}

	go func() {
		s.statusChan <- map[string]*dbus.UnitStatus{
			"b.service": {ActiveState: "inactive"},
		}
	}()
	if !reflect.DeepEqual(<-activeChan, []string{"a.service"}) {
		t.Error("Not equal")
	}
}
