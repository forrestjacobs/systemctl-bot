package status

import (
	"strings"

	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

type SubscriptionSet interface {
	Add(value string)
	Subscribe() (<-chan map[string]*dbus.UnitStatus, <-chan error)
}

type DiscordSession interface {
	UpdateGameStatus(idle int, name string) (err error)
}

func UpdateStatusFromUnits(discord DiscordSession, c *config.Config, set SubscriptionSet) {
	units := c.StatusUnits
	for _, unit := range units {
		set.Add(unit)
	}

	statusChan, errChan := set.Subscribe()
	go func() {
		for err := range errChan {
			println("Error subscribing to units: ", err)
		}
	}()

	go func() {
		activeStates := make(map[string]bool)
		for statuses := range statusChan {
			for name, status := range statuses {
				activeStates[name] = status.ActiveState == "active"
			}
			activeUnits := []string{}
			for _, unit := range units {
				if activeStates[unit] {
					activeUnits = append(activeUnits, unit)
				}
			}
			err := discord.UpdateGameStatus(0, strings.Join(activeUnits, ", "))
			if err != nil {
				println("Error listening to dbus events: ", err)
			}
		}
	}()
}
