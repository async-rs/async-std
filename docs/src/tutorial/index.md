# Tutorial: Writing a chat

Nothing is as simple as a chat server, right? Not quite, chat servers
already expose you to all the fun of asynchronous programming. How
do you handle clients connecting concurrently? How do you handle them disconnecting?
How do you distribute the messages?

In this tutorial, we will show you how to write one in `async-std`.

You can also find the tutorial in [our repository](https://github.com/async-rs/a-chat).

