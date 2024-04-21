package main

import (
	"context"
	"errors"
	"log"
	"os/exec"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/samber/lo"
)

func makeInteractionHandler(config *systemctlBotConfig, systemd *dbus.Conn) func(session *discordgo.Session, i *discordgo.InteractionCreate) {
	unitMap := lo.KeyBy(config.Units, func(unit *systemctlUnit) string {
		return unit.Name
	})

	return func(session *discordgo.Session, i *discordgo.InteractionCreate) {
		data := i.ApplicationCommandData()

		var name string
		var options []*discordgo.ApplicationCommandInteractionDataOption
		switch config.CommandType {
		case Single:
			subCommand := data.Options[0]
			name = subCommand.Name
			options = subCommand.Options
		case Multiple:
			name = data.Name
			options = data.Options
		}

		err := session.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
			Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
		})
		if err != nil {
			log.Println("Error responding to interaction: ", err)
			return
		}

		var content string

		switch name {
		case "start":
			content, err = startUnit(unitMap[options[0].StringValue()])
		case "stop":
			content, err = stopUnit(unitMap[options[0].StringValue()])
		case "restart":
			content, err = restartUnit(unitMap[options[0].StringValue()])
		case "status":
			if len(options) == 0 {
				content = getMultiStatus(getUnitsWithPermissions(config.Units, Status), systemd)
			} else {
				content, err = getStatus(unitMap[options[0].StringValue()], systemd)
			}
		default:
			err = errors.New("invalid command")
		}

		if err != nil {
			content = err.Error()
		}
		_, err = session.FollowupMessageCreate(i.Interaction, false, &discordgo.WebhookParams{
			Content: content,
		})
		if err != nil {
			log.Println("Error following up to interaction: ", err)
			return
		}
	}
}

func startUnit(unit *systemctlUnit) (string, error) {
	if !unit.HasPermissions(Start) {
		return "", errors.New("command is not allowed")
	}

	err := exec.Command("systemctl", "start", unit.Name).Run()
	if err != nil {
		return "", err
	}

	return "Started " + unit.Name, nil
}

func stopUnit(unit *systemctlUnit) (string, error) {
	if !unit.HasPermissions(Stop) {
		return "", errors.New("command is not allowed")
	}

	err := exec.Command("systemctl", "stop", unit.Name).Run()
	if err != nil {
		return "", err
	}

	return "Stopped " + unit.Name, nil
}

func restartUnit(unit *systemctlUnit) (string, error) {
	if !unit.HasPermissions(Stop, Start) {
		return "", errors.New("command is not allowed")
	}

	err := exec.Command("systemctl", "restart", unit.Name).Run()
	if err != nil {
		return "", err
	}

	return "Restarted " + unit.Name, nil
}

func getMultiStatus(units []*systemctlUnit, systemd *dbus.Conn) string {
	lines := lo.FilterMap(units, func(unit *systemctlUnit, _ int) (string, bool) {
		prop, err := systemd.GetUnitPropertyContext(context.Background(), unit.Name, "ActiveState")
		if err != nil {
			log.Println("Error fetching unit state: ", err)
		}
		val := prop.Value.Value().(string)
		return unit.Name + ": " + val, val != "inactive"
	})

	if len(lines) == 0 {
		return "Nothing is active"
	} else {
		return strings.Join(lines, "\n")
	}
}

func getStatus(unit *systemctlUnit, systemd *dbus.Conn) (string, error) {
	if !unit.HasPermissions(Status) {
		return "", errors.New("command is not allowed")
	}

	prop, err := systemd.GetUnitPropertyContext(context.Background(), unit.Name, "ActiveState")
	if err != nil {
		return "", err
	}

	return prop.Value.Value().(string), nil
}
