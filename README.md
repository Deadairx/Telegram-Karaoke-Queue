The idea behind this is I would start a session and users can join the session
by giving the Telegram Bot a session code, after that, they can share
youtube links and it would add it to a queue

The queue service needs to be able to play videos to a casted device

Probably gonna need to validate that the link is a valid youtube link first

#enhancement validate the link is not duplicate
#enhancement prioritize queue so that users who haven't gone in a while get queued up sooner

Needs crashing contingency plan
    Store user ids so that on server restart users don't need to enter the session code again
    Keep a history of links "played" incase one gets skipped for some reason


