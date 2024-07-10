package config

import (
	"errors"
	"fmt"
	"io"
	"os"
	"slices"
	"strconv"
	"strings"

	"github.com/BurntSushi/toml"
)

type Command string

const (
	StartCommand   Command = "start"
	StopCommand    Command = "stop"
	RestartCommand Command = "restart"
	StatusCommand  Command = "status"
)

type CommandType string

const (
	Single   CommandType = "single"
	Multiple CommandType = "multiple"
)

type Config struct {
	ApplicationID uint64
	DiscordToken  string
	GuildID       uint64
	CommandType   CommandType
	Units         map[Command][]string
}

type tomlConfig struct {
	ApplicationID uint64      `toml:"application_id"`
	DiscordToken  string      `toml:"discord_token"`
	GuildID       uint64      `toml:"guild_id"`
	CommandType   CommandType `toml:"command_type"`
	Units         []*unit     `toml:"units"`
}

type unit struct {
	Name        string
	Permissions []permission
}

type permission string

const (
	StartPermission  permission = "start"
	StopPermission   permission = "stop"
	StatusPermission permission = "status"
)

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

func getConfigErrors(config tomlConfig) error {
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

func hasEveryPermission(u *unit, permissions []permission) bool {
	for _, permission := range permissions {
		if !slices.Contains(u.Permissions, permission) {
			return false
		}
	}
	return true
}

func getUnitsWithPermissions(units []*unit, permissions ...permission) []string {
	value := []string{}
	for _, unit := range units {
		if hasEveryPermission(unit, permissions) {
			value = append(value, unit.Name)
		}
	}
	return value
}

func ReadConfig(r io.Reader) (*Config, error) {
	var config tomlConfig

	_, err := toml.NewDecoder(r).Decode(&config)
	if err != nil {
		return nil, err
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
		config.CommandType = CommandType(val)
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

	units := config.Units
	return &Config{
		ApplicationID: config.ApplicationID,
		DiscordToken:  config.DiscordToken,
		GuildID:       config.GuildID,
		CommandType:   config.CommandType,
		Units: map[Command][]string{
			StartCommand:   getUnitsWithPermissions(units, StartPermission),
			StopCommand:    getUnitsWithPermissions(units, StopPermission),
			RestartCommand: getUnitsWithPermissions(units, StartPermission, StopPermission),
			StatusCommand:  getUnitsWithPermissions(units, StatusPermission),
		},
	}, getConfigErrors(config)
}
