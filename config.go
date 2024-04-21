package main

import (
	"errors"
	"flag"
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/BurntSushi/toml"
	"github.com/samber/lo"
)

type unitPermission string

const (
	Start  unitPermission = "start"
	Stop   unitPermission = "stop"
	Status unitPermission = "status"
)

type commandType string

const (
	Single   commandType = "single"
	Multiple commandType = "multiple"
)

type systemctlUnit struct {
	Name        string
	Permissions []unitPermission
}

type systemctlBotConfig struct {
	ApplicationID uint64      `toml:"application_id"`
	DiscordToken  string      `toml:"discord_token"`
	GuildID       uint64      `toml:"guild_id"`
	CommandType   commandType `toml:"command_type"`
	Units         []*systemctlUnit
}

func (unit *systemctlUnit) HasPermissions(permissions ...unitPermission) bool {
	return lo.Every(unit.Permissions, permissions)
}

func getUnitsWithPermissions(units []*systemctlUnit, permissions ...unitPermission) []*systemctlUnit {
	return lo.Filter(units, func(unit *systemctlUnit, _ int) bool {
		return unit.HasPermissions(permissions...)
	})
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
			if permission != Start && permission != Stop && permission != Status {
				errs = append(errs, errors.New("invalid permission"))
			}
		}
	}

	return errors.Join(errs...)
}

func getConfig() (systemctlBotConfig, error) {
	var configPath string

	// TODO: add short version
	flag.StringVar(&configPath, "config", "/etc/systemctl-bot.toml", "path to config file")

	// TODO: Implement all of Clap's options
	flag.Parse()

	var config systemctlBotConfig

	_, err := toml.DecodeFile(configPath, &config)
	if err != nil {
		return systemctlBotConfig{}, err
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
