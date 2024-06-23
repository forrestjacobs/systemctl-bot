package main

import "github.com/bwmarrin/discordgo"

func makeStringOption(v string) *discordgo.ApplicationCommandInteractionDataOption {
	return &discordgo.ApplicationCommandInteractionDataOption{
		Type:  discordgo.ApplicationCommandOptionString,
		Value: v,
	}
}

type mockCall struct {
	name string
	args []any
}
