package handler

import (
	"context"
	"log"

	"github.com/bwmarrin/discordgo"
	"github.com/forrestjacobs/systemctl-bot/internal/config"
)

type DiscordSession interface {
	AddHandler(handler interface{}) func()
	InteractionRespond(interaction *discordgo.Interaction, resp *discordgo.InteractionResponse, options ...discordgo.RequestOption) error
	FollowupMessageCreate(interaction *discordgo.Interaction, wait bool, data *discordgo.WebhookParams, options ...discordgo.RequestOption) (*discordgo.Message, error)
}

type Systemd interface {
	StartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	StopUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
	RestartUnitContext(ctx context.Context, name string, mode string, ch chan<- string) (int, error)
}

type HandleContext struct {
	CommandName string
	Options     []*discordgo.ApplicationCommandInteractionDataOption
	Session     DiscordSession
	Interaction *discordgo.Interaction
	Systemd     Systemd
	Config      *config.Config
}

func (ctx *HandleContext) checkAllowed(unit string, permissions ...config.Permission) bool {
	allowed := ctx.Config.HasUnitWithPermissions(unit, permissions...)
	if !allowed {
		log.Println(ctx.CommandName + " is not an allowed command for " + unit)
	}
	return allowed
}

func logError(err error) {
	if err != nil {
		log.Println(err)
	}
}

func (ctx *HandleContext) deferResponse() bool {
	err := ctx.Session.InteractionRespond(ctx.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredChannelMessageWithSource,
	})
	logError(err)
	return err == nil
}

func (ctx *HandleContext) followUp(content string) {
	_, err := ctx.Session.FollowupMessageCreate(ctx.Interaction, false, &discordgo.WebhookParams{
		Content: content,
	})
	logError(err)
}

type Command struct {
	Name        string
	Description string
	Build       func(c *config.Config) ([]*discordgo.ApplicationCommandOption, bool)
	Handle      func(ctx *HandleContext)
}
