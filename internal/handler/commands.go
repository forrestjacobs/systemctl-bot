package handler

import (
	"context"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

func getUnitOption(description string, units []string) *discordgo.ApplicationCommandOption {
	choices := []*discordgo.ApplicationCommandOptionChoice{}
	for _, unit := range units {
		choices = append(choices, &discordgo.ApplicationCommandOptionChoice{
			Name:  strings.TrimSuffix(unit, ".service"),
			Value: unit,
		})
	}
	return &discordgo.ApplicationCommandOption{
		Name:        "unit",
		Type:        discordgo.ApplicationCommandOptionString,
		Description: description,
		Required:    true,
		Choices:     choices,
	}
}

func getSystemdResponse(doneString string, resultChan <-chan string, err error) string {
	if err != nil {
		return err.Error()
	}

	result := <-resultChan
	if result == "done" {
		return doneString
	}

	return result
}

var start = Command{
	Name:        "start",
	Description: "Start units",
	Build: func(c *config.Config) ([]*discordgo.ApplicationCommandOption, bool) {
		units := c.GetUnitsWithPermissions(config.StartPermission)
		if len(units) == 0 {
			return nil, false
		}
		return []*discordgo.ApplicationCommandOption{getUnitOption("The unit to start", units)}, true
	},
	Handle: func(ctx *HandleContext) {
		unit := ctx.Options[0].StringValue()
		if ctx.checkAllowed(unit, config.StartPermission) && ctx.deferResponse() {
			resultChan := make(chan string)
			_, err := ctx.Systemd.StartUnitContext(context.Background(), unit, "replace", resultChan)
			ctx.followUp(getSystemdResponse("Started "+unit, resultChan, err))
		}
	},
}

var stop = Command{
	Name:        "stop",
	Description: "Stop units",
	Build: func(c *config.Config) ([]*discordgo.ApplicationCommandOption, bool) {
		units := c.GetUnitsWithPermissions(config.StopPermission)
		if len(units) == 0 {
			return nil, false
		}
		return []*discordgo.ApplicationCommandOption{getUnitOption("The unit to stop", units)}, true
	},
	Handle: func(ctx *HandleContext) {
		unit := ctx.Options[0].StringValue()
		if ctx.checkAllowed(unit, config.StopPermission) && ctx.deferResponse() {
			resultChan := make(chan string)
			_, err := ctx.Systemd.StopUnitContext(context.Background(), unit, "replace", resultChan)
			ctx.followUp(getSystemdResponse("Stopped "+unit, resultChan, err))
		}
	},
}

var restart = Command{
	Name:        "restart",
	Description: "Restart units",
	Build: func(c *config.Config) ([]*discordgo.ApplicationCommandOption, bool) {
		units := c.GetUnitsWithPermissions(config.StartPermission, config.StopPermission)
		if len(units) == 0 {
			return nil, false
		}
		return []*discordgo.ApplicationCommandOption{getUnitOption("The unit to restart", units)}, true
	},
	Handle: func(ctx *HandleContext) {
		unit := ctx.Options[0].StringValue()
		if ctx.checkAllowed(unit, config.StartPermission, config.StopPermission) && ctx.deferResponse() {
			resultChan := make(chan string)
			_, err := ctx.Systemd.RestartUnitContext(context.Background(), unit, "replace", resultChan)
			ctx.followUp(getSystemdResponse("Restarted "+unit, resultChan, err))
		}
	},
}

var Commands []*Command = []*Command{&start, &stop, &restart}
