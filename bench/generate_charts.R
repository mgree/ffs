#!/usr/bin/env Rscript
library(ggplot2)

args = commandArgs(trailingOnly=TRUE)

bench <- data.frame(read.csv(args[1]))
bench$activity <- factor(bench$activity, levels = c("reading", "loading", "saving", "writing"))
benchPlot <- ggplot(bench) +
  xlab("File size (MiB)") + ylab("Time (ms, log10)") + scale_colour_discrete(name = "Activity") +
  scale_x_continuous(limits=c(0,3100), breaks=c(0,1024,2048,3072), labels=c("0MiB","1MiB","2MiB","3MiB")) +
  scale_y_continuous(breaks=c(0,2,4,6,8,10),labels=c("0ms", "0.01ms", "0.1ms", "1ms", "10ms", "100ms")) +
  geom_point(aes(size/1024,log(ns,base=10),colour=activity))

ggsave(gsub("log","png",args[1]), benchPlot, width=4.5, height=4.5)

micro <- data.frame(read.csv(args[2]))
micro$activity <- factor(micro$activity, levels = c("reading", "loading", "saving", "writing"))
microPlot <- ggplot(micro) +
  xlab("JSON value size") + ylab("Time (ms, log10)") + scale_colour_discrete(name = "Activity") + scale_shape_discrete(name = "Kind") +
  scale_x_continuous(limits=c(0,8), breaks=c(0,2,4,6,8), labels=c("1", "4", "16", "64", "256")) +
  scale_y_continuous(breaks=c(0,2,4,6,8,10),labels=c("0ms", "0.01ms", "0.1ms", "1ms", "10ms", "100ms")) +
  geom_point(aes(log(magnitude,base=2),log(ns,base=10),colour=activity,shape=kind)) +
  facet_wrap( ~ direction, labeller = as_labeller(c(`deep` = "Deep { { ... } }", `wide` = "Wide { ..., ... }")))

ggsave(gsub("log","png",args[2]), microPlot, width=4.5, height=4.5)