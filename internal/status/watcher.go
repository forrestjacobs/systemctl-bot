package status

import (
	"log"
	"strings"

	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
	"github.com/samber/lo"
)

type SubscriptionSet interface {
	Add(value string)
	Subscribe() (<-chan map[string]*dbus.UnitStatus, <-chan error)
}

type DiscordSession interface {
	UpdateGameStatus(idle int, name string) (err error)
}

func UpdateStatusFromUnits(discord DiscordSession, c *config.Config, set SubscriptionSet) <-chan error {
	units := c.Units[config.StatusCommand]
	for _, unit := range units {
		set.Add(unit)
	}

	statusChan, errChan := set.Subscribe()

	go func() {
		activeStates := make(map[string]bool)
		for statuses := range statusChan {
			for name, status := range statuses {
				activeStates[name] = status.ActiveState == "active"
			}
			activeUnits := lo.FilterMap(units, func(unit string, _ int) (string, bool) {
				return unit, activeStates[unit]
			})
			err := discord.UpdateGameStatus(0, strings.Join(activeUnits, ", "))
			if err != nil {
				log.Println("Error updating status: ", err)
			}
		}
	}()
	return errChan
}
