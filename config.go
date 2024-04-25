package main

import (
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"github.com/BurntSushi/toml"
)

type commandType string

const (
	Single   commandType = "single"
	Multiple commandType = "multiple"
)

type systemctlBotConfig struct {
	ApplicationID uint64      `toml:"application_id"`
	DiscordToken  string      `toml:"discord_token"`
	GuildID       uint64      `toml:"guild_id"`
	CommandType   commandType `toml:"command_type"`
	Units         []*systemctlUnit
}

func lookupUint64Env(key string) (uint64, bool) {
	strVal, exists := os.LookupEnv(key)
	if !exists {
		return 0, false
	}

	val, err := strconv.ParseUint(strVal, 10, 64)
	if err != nil {
		fmt.Println("Warning: could not parse", key, ":", err)
		return 0, false
	}

	return val, true
}

func getConfigErrors(config systemctlBotConfig) error {
	errs := make([]error, 0)

	if config.ApplicationID == 0 {
		errs = append(errs, errors.New("missing application_id"))
	}
	if config.DiscordToken == "" {
		errs = append(errs, errors.New("missing discord_token"))
	}
	if config.GuildID == 0 {
		errs = append(errs, errors.New("missing guild_id"))
	}
	if config.CommandType != Single && config.CommandType != Multiple {
		errs = append(errs, errors.New("invalid command_type"))
	}
	if len(config.Units) == 0 {
		errs = append(errs, errors.New("missing units"))
	}

	for _, unit := range config.Units {
		if len(unit.Name) == 0 {
			errs = append(errs, errors.New("missing unit name"))
		}
		if len(unit.Permissions) == 0 {
			errs = append(errs, errors.New("missing unit permissions"))
		}
		for _, permission := range unit.Permissions {
			if permission != StartPermission && permission != StopPermission && permission != StatusPermission {
				errs = append(errs, errors.New("invalid unit permission"))
			}
		}
	}

	return errors.Join(errs...)
}

func getConfig() (systemctlBotConfig, error) {
	var path string

	// TODO: add short version
	flag.StringVar(&path, "config", "/etc/systemctl-bot.toml", "path to config file")

	// TODO: Implement all of Clap's options
	flag.Parse()

	reader, err := os.Open(path)
	if err != nil {
		return systemctlBotConfig{}, err
	}
	defer reader.Close()

	return readConfig(reader)
}

func readConfig(r io.Reader) (systemctlBotConfig, error) {
	var config systemctlBotConfig

	_, err := toml.NewDecoder(r).Decode(&config)
	if err != nil {
		return config, err
	}

	if val, present := lookupUint64Env("SBOT_APPLICATION_ID"); present {
		config.ApplicationID = val
	}
	if val, present := lookupUint64Env("SBOT_GUILD_ID"); present {
		config.GuildID = val
	}
	if val, present := os.LookupEnv("SBOT_DISCORD_TOKEN"); present {
		config.DiscordToken = val
	}
	if val, present := os.LookupEnv("SBOT_COMMAND_TYPE"); present {
		config.CommandType = commandType(val)
	}

	if config.CommandType == "" {
		config.CommandType = Single
	}
	for i, unit := range config.Units {
		name := unit.Name
		if len(name) > 0 && !strings.Contains(name, ".") {
			config.Units[i].Name = name + ".service"
		}
	}

	return config, getConfigErrors(config)
}
