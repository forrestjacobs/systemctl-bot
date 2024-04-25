package main

import (
	"context"
	"log"
	"os"
	"strconv"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/samber/lo"
)

type exitErrorCode int

const (
	ConfigReadError          exitErrorCode = 10
	CommandConstructionError exitErrorCode = 11

	SystemdOpenConnectionError exitErrorCode = 20

	DiscordCreateSessionError  exitErrorCode = 30
	DiscordOpenConnectionError exitErrorCode = 31
	DiscordSetCommandError     exitErrorCode = 32
)

func dieOnError(err error, code exitErrorCode) {
	if err != nil {
		log.Println(err)
		os.Exit(int(code))
	}
}

func main() {
	config, err := getConfig()
	dieOnError(err, ConfigReadError)
	commandUnits := getCommandUnits(config.Units)

	commands, err := getCommands(commandUnits, config.CommandType)
	dieOnError(err, CommandConstructionError)

	conn, err := dbus.NewSystemdConnectionContext(context.Background())
	dieOnError(err, SystemdOpenConnectionError)
	defer conn.Close()
	systemd := systemdImpl{conn: conn}

	discord, err := discordgo.New("Bot " + config.DiscordToken)
	dieOnError(err, DiscordCreateSessionError)

	discord.AddHandler(makeInteractionHandler(commandUnits, &systemd))
	dieOnError(discord.Open(), DiscordOpenConnectionError)
	defer discord.Close()

	applicationID := strconv.FormatUint(config.ApplicationID, 10)
	guildID := strconv.FormatUint(config.GuildID, 10)
	_, err = discord.ApplicationCommandBulkOverwrite(applicationID, guildID, commands)
	dieOnError(err, DiscordSetCommandError)

	activeUnitsChan, errChan := systemd.subscribeToActiveUnits(commandUnits[StatusCommand])

	for {
		select {
		case activeUnits := <-activeUnitsChan:
			activeList := lo.FilterMap(commandUnits[StatusCommand], func(unit string, _ int) (string, bool) {
				return unit, activeUnits[unit]
			})
			err := discord.UpdateGameStatus(0, strings.Join(activeList, ", "))
			if err != nil {
				log.Println("Error updating status: ", err)
			}
		case err := <-errChan:
			log.Println("Error listening to dbus events: ", err)
		}
	}
}
