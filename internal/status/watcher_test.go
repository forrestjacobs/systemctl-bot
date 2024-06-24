package status

import (
	"reflect"
	"testing"

	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

type mockSubscriptionSet struct {
	mockCalls  [][]any
	statusChan chan map[string]*dbus.UnitStatus
}

func (m *mockSubscriptionSet) Add(value string) {
	m.mockCalls = append(m.mockCalls, []any{"Add", value})
}

func (m *mockSubscriptionSet) Subscribe() (<-chan map[string]*dbus.UnitStatus, <-chan error) {
	m.mockCalls = append(m.mockCalls, []any{"Subscribe"})
	return m.statusChan, nil
}

type mockDiscordSession struct {
	updateChan chan []any
}

func (d *mockDiscordSession) UpdateGameStatus(idle int, name string) (err error) {
	d.updateChan <- []any{idle, name}
	return nil
}

func TestUpdateStatusFromUnits(t *testing.T) {
	s := mockSubscriptionSet{
		statusChan: make(chan map[string]*dbus.UnitStatus),
	}
	d := mockDiscordSession{
		updateChan: make(chan []any),
	}

	UpdateStatusFromUnits(&d, &config.Config{
		Units: map[config.Command][]string{
			config.StatusCommand: {"a.service", "b.service"},
		},
	}, &s)

	if !reflect.DeepEqual(s.mockCalls, [][]any{
		{"Add", "a.service"},
		{"Add", "b.service"},
		{"Subscribe"},
	}) {
		t.Error("Not equal")
	}

	s.statusChan <- map[string]*dbus.UnitStatus{
		"a.service": {ActiveState: "active"},
	}
	call := <-d.updateChan
	if !reflect.DeepEqual(call, []any{0, "a.service"}) {
		t.Error("Not equal")
	}

	s.statusChan <- map[string]*dbus.UnitStatus{
		"b.service": {ActiveState: "active"},
	}
	call = <-d.updateChan
	if !reflect.DeepEqual(call, []any{0, "a.service, b.service"}) {
		t.Error("Not equal")
	}

	s.statusChan <- map[string]*dbus.UnitStatus{
		"b.service": {ActiveState: "inactive"},
	}
	call = <-d.updateChan
	if !reflect.DeepEqual(call, []any{0, "a.service"}) {
		t.Error("Not equal")
	}
}
