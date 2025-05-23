package config_test

import (
	"os"
	"reflect"
	"strings"
	"testing"

	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

const baseConfigToml = `
application_id = 1
guild_id = 2
discord_token = "a"
command_type = "single"

[[units]]
name = "s.service"
permissions = [ "status" ]
`

func getBaseConfig() *config.Config {
	return &config.Config{
		ApplicationID: 1,
		GuildID:       2,
		DiscordToken:  "a",
		CommandType:   config.Single,
		CommandUnits: map[config.Command][]string{
			config.StartCommand:   {},
			config.StopCommand:    {},
			config.RestartCommand: {},
		},
		StatusUnits: []string{"s.service"},
	}
}

func TestReadConfig(t *testing.T) {
	config, err := config.ReadConfig(strings.NewReader(baseConfigToml))

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}
	if !reflect.DeepEqual(config, getBaseConfig()) {
		t.Error("Not equal")
	}
}

func TestReadConfigWithInvalidEnvironmentVariables(t *testing.T) {
	os.Setenv("SBOT_APPLICATION_ID", "one")
	os.Setenv("SBOT_GUILD_ID", "two")
	config, err := config.ReadConfig(strings.NewReader(baseConfigToml))
	os.Clearenv()

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}
	if !reflect.DeepEqual(config, getBaseConfig()) {
		t.Error("Not equal")
	}
}

func TestReadConfigWithEnvironmentVariables(t *testing.T) {
	os.Setenv("SBOT_APPLICATION_ID", "10")
	os.Setenv("SBOT_GUILD_ID", "20")
	os.Setenv("SBOT_DISCORD_TOKEN", "Z")
	os.Setenv("SBOT_COMMAND_TYPE", "multiple")
	c, err := config.ReadConfig(strings.NewReader(baseConfigToml))
	os.Clearenv()

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}
	baseConfig := getBaseConfig()
	if !reflect.DeepEqual(c, &config.Config{
		ApplicationID: 10,
		GuildID:       20,
		DiscordToken:  "Z",
		CommandType:   config.Multiple,
		CommandUnits:  baseConfig.CommandUnits,
		StatusUnits:   baseConfig.StatusUnits,
	}) {
		t.Error("Not equal")
	}
}

func TestReadConfigSuppliesDefaults(t *testing.T) {
	c, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2
		discord_token = "a"

		[[units]]
		name = "s"
		permissions = [ "status" ]

		[[units]]
		name = "t.timer"
		permissions = [ "status" ]
	`))

	if err != nil {
		t.Fatalf("Unexpected error %v", err)
	}
	if !reflect.DeepEqual(c, &config.Config{
		ApplicationID: 1,
		GuildID:       2,
		DiscordToken:  "a",
		CommandType:   config.Single,
		CommandUnits: map[config.Command][]string{
			config.StartCommand:   {},
			config.StopCommand:    {},
			config.RestartCommand: {},
		},
		StatusUnits: []string{"s.service", "t.timer"},
	}) {
		t.Error("Not equal")
	}
}

func TestReadBadToml(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader("bad bad not good"))
	if err == nil {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithoutApplicationID(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		guild_id = 2
		discord_token = "a"

		[[units]]
		name = "s.service"
		permissions = [ "status" ]
	`))
	if err.Error() != "missing application_id" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithoutDiscordToken(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2

		[[units]]
		name = "s.service"
		permissions = [ "status" ]
	`))
	if err.Error() != "missing discord_token" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithoutGuildID(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		discord_token = "a"

		[[units]]
		name = "s.service"
		permissions = [ "status" ]
	`))
	if err.Error() != "missing guild_id" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithInvalidCommandType(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2
		discord_token = "a"
		command_type = "invalid"

		[[units]]
		name = "s.service"
		permissions = [ "status" ]
	`))
	if err.Error() != "invalid command_type" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithoutUnits(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2
		discord_token = "a"
	`))
	if err.Error() != "missing units" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithoutUnitName(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2
		discord_token = "a"

		[[units]]
		permissions = [ "status" ]
	`))
	if err.Error() != "missing unit name" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithoutUnitPermissions(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2
		discord_token = "a"

		[[units]]
		name = "s.service"
	`))
	if err.Error() != "missing unit permissions" {
		t.Fatalf("Unexpected error %v", err)
	}
}

func TestReadConfigWithInvalidUnitPermission(t *testing.T) {
	_, err := config.ReadConfig(strings.NewReader(`
		application_id = 1
		guild_id = 2
		discord_token = "a"

		[[units]]
		name = "s.service"
		permissions = [ "invalid" ]
	`))
	if err.Error() != "invalid unit permission" {
		t.Fatalf("Unexpected error %v", err)
	}
}
