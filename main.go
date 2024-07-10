package main

import (
	"context"
	"flag"
	"log"
	"os"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/builder"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
	"github.com/forrestjacobs/systemctl-bot/internal/handler"
	"github.com/forrestjacobs/systemctl-bot/internal/status"
)

type exitErrorCode int

const (
	ConfigReadError exitErrorCode = 10

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

func getConfig() (*config.Config, error) {
	const defaultPath = "/etc/systemctl-bot.toml"

	var path string
	flag.StringVar(&path, "config", defaultPath, "path to config file")
	flag.StringVar(&path, "c", defaultPath, "path to config file (shorthand)")

	// TODO: Implement all of Clap's options
	flag.Parse()

	configFileReader, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer configFileReader.Close()

	return config.ReadConfig(configFileReader)
}

func main() {
	c, err := getConfig()
	dieOnError(err, ConfigReadError)

	conn, err := dbus.NewSystemdConnectionContext(context.Background())
	dieOnError(err, SystemdOpenConnectionError)
	defer conn.Close()

	discord, err := discordgo.New("Bot " + c.DiscordToken)
	dieOnError(err, DiscordCreateSessionError)

	handler.AddHandler(discord, conn, c)
	dieOnError(discord.Open(), DiscordOpenConnectionError)
	defer discord.Close()

	dieOnError(builder.RegisterCommands(discord, c), DiscordSetCommandError)

	errChan := status.UpdateStatusFromUnits(discord, c, conn.NewSubscriptionSet())
	for err := range errChan {
		log.Println("Error listening to dbus events: ", err)
	}
}
