# Holly
A programmable bot for Messenger, using Selenium

## Setup

0. Install cargo, rust, and geckodriver
1. Clone the repository
2. Run `cargo run --release` to create a config file
3. Edit the config file to your liking
4. Run `cargo run --release` again to start the bot

This will only start a bot capable of responding to/sending messages, but will do nothing right now.

## Usage

Connect to the TCP socket defined in the `config.toml`.
Holly will send messages to all clients in the form of JSON, that looks like this:
```json
{
    "sender": "username",
    "content": "Ping!",
    "chat_id": "1234567890"
}
```
You can respond with an identical JSON:
```json
{
    "sender": "", // can be left blank, but must be included for parsing
    "content": "Pong!",
    "chat_id": "1234567890"
}
```

Holly also supports commands by TCP for logging and control. 
In the `sender` field, you can send the following values:
- `"<screenshot>"`: Takes a screenshot and saves it to `log/<timestamp>.png`
- `"<html>"`: Dumps the current HTML on the page
- `"<restart>"`: Restarts the bot
- `"<refresh>"`: Refreshes the page

### Example

```json
{
    "sender": "<restart>",
    "content": "", // again, these fields can be blank but must be included
    "chat_id": ""
}
```

## Library
For your convenience, there is a simple library that abstracts the boilerplate code for writing code for Holly.
You can view the library at `holly.py` and place it in your PYTHONPATH.

### Example

```python
import holly
import time

def main():
    parser = holly.HollyParser()

    while True:
        try:
            client = holly.HollyClient()
            print('Connected to Holly')
            while True:
                raw_msg = client.recv()
                msg = raw_msg.parse(parser)
                if msg.match("ping"):
                    client.send(holly.HollyMessage(content="pong", chat_id=raw_msg.chat_id, sender=''))

        except holly.HollyError as e:
            print(f"Error: {e}")

        print('Disconnected from Holly socket')
        time.sleep(30)

if __name__ == "__main__":
    main()
```

## Examples
You can find more examples in the `children` folder, which are all written using the library.
