library(ggplot2)

setwd("~/ffs/bench")
sizes <- data.frame(read.csv("20210727_run.log"))
names(sizes)

ggplot(sizes) + geom_point(aes(size,ns,colour=source), data=subset(sizes, activity %in% "reading json"))
ggplot(sizes) + geom_point(aes(size,ns,colour=source), data=subset(sizes, activity %in% "inodes"))
ggplot(sizes) + geom_point(aes(size,ns,colour=source), data=subset(sizes, activity %in% "saving"))
ggplot(sizes) + geom_point(aes(size,ns,colour=source), data=subset(sizes, activity %in% "writing"))
