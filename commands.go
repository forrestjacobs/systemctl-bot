package main

import (
	"context"
	"log"
	"slices"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/samber/lo"
)

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

var commandHandlers = map[command]func(ctx *commandCtx, runner *commandRunnerImpl){
	StartCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(StartCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.StartUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Started "+unit, resultChan, err))
		}
	},

	StopCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(StopCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.StopUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Stopped "+unit, resultChan, err))
		}
	},

	RestartCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(RestartCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.RestartUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Restarted "+unit, resultChan, err))
		}
	},

	StatusCommand: func(ctx *commandCtx, runner *commandRunnerImpl) {
		if len(ctx.options) == 0 {
			lines := lo.FilterMap(runner.commandUnits[StatusCommand], func(unit string, _ int) (string, bool) {
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
			if runner.checkAllowed(StatusCommand, unit) {
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
	systemd      systemd
	commandUnits map[command][]string
}

func (runner *commandRunnerImpl) checkAllowed(command command, value string) bool {
	allowed := slices.Contains(runner.commandUnits[command], value)
	if !allowed {
		log.Println(string(command) + " is not an allowed command for " + value)
	}
	return allowed
}

func (runner *commandRunnerImpl) run(ctx *commandCtx) {
	if handler, ok := commandHandlers[command(ctx.commandName)]; ok {
		handler(ctx, runner)
	}
}
