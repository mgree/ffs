library(ggplot2)

setwd("~/ffs/bench")
sizes <- data.frame(read.csv("20210727_run.log"))

#ggplot(subset(sizes, activity %in% "reading json")) + geom_smooth(method="lm", aes(x=size, y=ns), formula=(y ~ x), se=FALSE, linetype = 1) + geom_point(aes(size,ns,colour=source))
#ggplot(subset(sizes, activity %in% "inodes")) + geom_smooth(method="lm", aes(x=size, y=ns), formula=(y ~ x), se=FALSE, linetype = 1) + geom_point(aes(size,ns,colour=source))
#ggplot(subset(sizes, activity %in% "saving")) + geom_smooth(method="lm", aes(x=size, y=ns), formula=(y ~ x), se=FALSE, linetype = 1) + geom_point(aes(size,ns,colour=source))
#ggplot(subset(sizes, activity %in% "writing")) + geom_smooth(method="lm", aes(x=size, y=ns), formula=(y ~ x), se=FALSE, linetype = 1) + geom_point(aes(size,ns,colour=source))

ggplot(sizes) +
  xlab("File size (MiB)") + ylab("Time (ms, log10)") + scale_colour_discrete(name = "Activity") +
  scale_x_continuous(limits=c(0,3100), breaks=c(0,1024,2048,3072), labels=c("0MiB","1MiB","2MiB","3MiB")) +
  scale_y_continuous(breaks=c(0,2,4,6,8,10),labels=c("0ms", "0.01ms", "0.1ms", "1ms", "10ms", "100ms")) +
  geom_point(aes(size/1024,log(ns,base=10),colour=activity))

micro <- data.frame(read.csv("20210727_micro.log"))
ggplot(micro) +
  xlab("JSON value size") + ylab("Time (ms, log10)") + scale_colour_discrete(name = "Activity") +
  scale_x_continuous(limits=c(0,8), breaks=c(0,2,4,6,8), labels=c("1", "4", "16", "64", "256")) +
  scale_y_continuous(breaks=c(0,2,4,6,8,10),labels=c("0ms", "0.01ms", "0.1ms", "1ms", "10ms", "100ms")) +
  geom_point(aes(log(magnitude,base=2),log(ns,base=10),colour=activity)) +
  facet_wrap( ~ direction, labeller = as_labeller(c(`deep` = "Deep { { ... } }", `wide` = "Wide { ..., ... }")))
