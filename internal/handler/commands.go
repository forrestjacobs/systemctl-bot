package handler

import (
	"context"
	"log"
	"strings"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
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

var commandHandlers = map[config.Command]func(ctx *commandCtx, runner *commandRunner){
	config.StartCommand: func(ctx *commandCtx, runner *commandRunner) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(config.StartCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.StartUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Started "+unit, resultChan, err))
		}
	},

	config.StopCommand: func(ctx *commandCtx, runner *commandRunner) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(config.StopCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.StopUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Stopped "+unit, resultChan, err))
		}
	},

	config.RestartCommand: func(ctx *commandCtx, runner *commandRunner) {
		unit := ctx.options[0].StringValue()
		if runner.checkAllowed(config.RestartCommand, unit) && deferResponse(ctx) {
			resultChan := make(chan string)
			_, err := runner.systemd.RestartUnitContext(context.Background(), unit, "replace", resultChan)
			followUp(ctx, getSystemdResponse("Restarted "+unit, resultChan, err))
		}
	},

	config.StatusCommand: func(ctx *commandCtx, runner *commandRunner) {
		if len(ctx.options) == 0 {
			lines := []string{}
			for _, unit := range runner.units[config.StatusCommand] {
				prop, err := runner.systemd.GetUnitPropertyContext(context.Background(), unit, "ActiveState")
				if err != nil {
					log.Println("Error fetching unit state: ", err)
					lines = append(lines, unit+": error getting status")
					continue
				}
				val := prop.Value.Value().(string)
				if val != "inactive" {
					lines = append(lines, unit+": "+val)
				}
			}

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
