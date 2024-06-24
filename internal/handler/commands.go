package handler

import (
	"context"
	"log"
	"slices"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/coreos/go-systemd/v22/dbus"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
	"github.com/samber/lo"
)

type systemd interface {
	StartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	StopUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	RestartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	GetUnitPropertyContext(ctx context.Context, unit string, propertyName string) (*dbus.Property, error)
}

func logError(err error) {
	if err != nil {
		log.Println(err)
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

func respond(ctx *commandCtx, content string) {
	err := ctx.session.InteractionRespond(ctx.interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
		},
	})
	logError(err)
}

func deferResponse(ctx *commandCtx) bool {
	err := ctx.session.InteractionRespond(ctx.interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	})
	logError(err)
	return err == nil
}

func followUp(ctx *commandCtx, content string) {
	_, err := ctx.session.FollowupMessageCreate(ctx.interaction, false, &discordgo.WebhookParams{
		Content: content,
	})
	logError(err)
}

var commandHandlers = map[config.Command]func(ctx *commandCtx, runner *commandRunnerImpl){
	config.StartCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(config.StartCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.StartUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Started "+unit, resultChan, err))
		}
	},

	config.StopCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(config.StopCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.StopUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Stopped "+unit, resultChan, err))
		}
	},

	config.RestartCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(config.RestartCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.RestartUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Restarted "+unit, resultChan, err))
		}
	},

	config.StatusCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		if len(ctx.options) == 0 {
			lines := lo.FilterMap(runner.units[config.StatusCommand], func(unit string, _ int) (string, bool) {
				prop, err := runner.systemd.GetUnitPropertyContext(context.Background(), unit, "ActiveState")
				if err != nil {
					log.Println("Error fetching unit state: ", err)
					return unit + ": error getting status", true
				}
				val := prop.Value.Value().(string)
				return unit + ": " + val, val != "inactive"
			})

			if len(lines) == 0 {
				respond(ctx, "Nothing is active")
			} else {
				respond(ctx, strings.Join(lines, "\n"))
			}
		} else {
			unit := ctx.options[0].StringValue()
			if runner.checkAllowed(config.StatusCommand, unit) {
				prop, err := runner.systemd.GetUnitPropertyContext(context.Background(), unit, "ActiveState")
				if err != nil {
					respond(ctx, err.Error())
				} else {
					respond(ctx, prop.Value.Value().(string))
				}
			}
		}
	},
}

type commandRunnerImpl struct {
	systemd systemd
	units   map[config.Command][]string
}

func (runner *commandRunnerImpl) checkAllowed(command config.Command, value string) bool {
	allowed := slices.Contains(runner.units[command], value)
	if !allowed {
		log.Println(string(command) + " is not an allowed command for " + value)
	}
	return allowed
}

func (runner *commandRunnerImpl) run(ctx *commandCtx) {
	if handler, ok := commandHandlers[config.Command(ctx.commandName)]; ok {
		handler(ctx, runner)
	}
}
