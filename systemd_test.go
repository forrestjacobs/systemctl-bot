package main

import (
	"reflect"
	"sort"
	"testing"

	"github.com/coreos/go-systemd/v22/dbus"
)

func TestSubscribeToUnits(t *testing.T) {
	set := subscribeToUnits(&dbus.Conn{}, []string{"a.service", "b.service"})

	setElems := set.Values()
	sort.Slice(setElems, func(i, j int) bool { return setElems[i] < setElems[j] })

	if !reflect.DeepEqual(setElems, []string{"a.service", "b.service"}) {
		t.Error("Not equal")
	}
}

func TestTransformStatusChanToActiveList(t *testing.T) {
	statusChan := make(chan map[string]*dbus.UnitStatus)
	activeChan := transformStatusChanToActiveList([]string{"a.service", "b.service"}, statusChan)

	go func() {
		statusChan <- map[string]*dbus.UnitStatus{
			"a.service": {ActiveState: "active"},
		}
	}()
	if !reflect.DeepEqual(<-activeChan, []string{"a.service"}) {
		t.Error("Not equal")
	}

	go func() {
		statusChan <- map[string]*dbus.UnitStatus{
			"b.service": {ActiveState: "active"},
		}
	}()
	if !reflect.DeepEqual(<-activeChan, []string{"a.service", "b.service"}) {
		t.Error("Not equal")
	}

	go func() {
		statusChan <- map[string]*dbus.UnitStatus{
			"b.service": {ActiveState: "inactive"},
		}
	}()
	if !reflect.DeepEqual(<-activeChan, []string{"a.service"}) {
		t.Error("Not equal")
	}
}
