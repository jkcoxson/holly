"""
Client library for writing Holly plugins
Authored by Jackson Coxson
"""

import json
import socket
import re
from typing import Union
import itertools

DEFAULT_JUNK = [
    'a',
    'an',
    'are',
    'as',
    'is',
    'the'
]

class HollyError(Exception):
    """Exception raised for errors in the Holly module.

    Attributes:
        message -- explanation of the error
    """

    def __init__(self, message="Holly Error"):
        """
        Initializes the HollyError instance.

        Args:
            message (str): Explanation of the error.
        """
        self.message = message
        super().__init__(self.message)

class ParsedHollyMessage:
    """Represents a parsed Holly message.

    Attributes:
        content (list[str]): The content of the message.
        chat_id: Identifier of the chat the message belongs to.
        sender: Sender of the message.
        targeted (bool): Indicates if the message is targeted at Holly.
    """

    def __init__(self, content: list[str], chat_id, sender, targeted):
        self.content = content
        self.chat_id = chat_id
        self.sender = sender
        self.target = targeted

    def __str__(self) -> str:
        return f"ParsedHollyMessage: {self.content}, {self.chat_id}, {self.sender}, {self.target}"

    def __repr__(self) -> str:
        return self.__str__()

    def match(self, test: Union[str, list[str]], lower=True):
        """Checks if the message content matches a test string or list of strings.

        Args:
            test: String or list of strings to match against the message content.
            lower (bool): If True, performs case-insensitive matching.

        Returns:
            bool: True if the content matches the test, False otherwise.
        """
        c = self.content
        if lower:
            if isinstance(test, str):
                test = test.lower()
            else:
                test = [x.lower() for x in test]
            c = [x.lower() for x in c]

        if isinstance(test, str):
            test = test.split()
        return c == test

    def loose_match(self, test: Union[str, list[str]], lower=True) -> bool:
        """Checks if the input string or list of strings appears anywhere in the message.

        Args:
            test (Union[str, List[str]]): The string or list of strings to search for.
            lower (bool): If True, performs case-insensitive matching. Default is False.

        Returns:
            bool: True if the input appears anywhere in the message, False otherwise.
        """
        content = self.content
        if lower:
            content = [x.lower() for x in content]

        if isinstance(test, str):
            test = test.split()

        if len(test) > len(content):
            return False

        for i in range(len(content) - len(test) + 1):
            if all(content[i + j].lower() == test[j].lower() for j in range(len(test))):
                return True
        return False


    def is_targeted(self):
        """Checks if the message is targeted at Holly.

        Returns:
            bool: True if the message is targeted at Holly, False otherwise.
        """
        return self.target


class HollyParser:
    """Parses messages for Holly commands.

    Attributes:
        junk_list (list[str]): List of words to ignore during parsing.
        remove_punctuation (bool): If True, removes punctuation from messages.
        name (str): Name of the bot.
        mention_name (str): Mention name of the bot.
    """

    def __init__(
        self,
        junk_list=None,
        remove_punctuation=True,
        name="Holly",
        mention_name="Holly Coxson",
    ):
        if junk_list is None:
            junk_list = DEFAULT_JUNK
        self.junk_list = junk_list
        if remove_punctuation:
            self.remove_punctuation = re.compile(r'[^a-zA-Z0-9\s]')
        self.name = name
        self.mention_name = mention_name

    def parse(self, content: str, chat_id, sender) -> ParsedHollyMessage:
        """Parses a message and returns a ParsedHollyMessage object.

        Args:
            content (str): The content of the message.
            chat_id: Identifier of the chat the message belongs to.
            sender: Sender of the message.

        Returns:
            ParsedHollyMessage: The parsed message object.
        """
        targeted = False
        if content.lower().startswith(self.name.lower()):
            targeted = True
            content = content[len(self.name):].strip()
        if '@' + self.mention_name in content:
            targeted = True
            content = content.replace('@' + self.mention_name, '').strip()

        if self.remove_punctuation:
            content = self.remove_punctuation.sub('', content)

        split_content = content.split()
        split_content = list(itertools.filterfalse(lambda x: x.lower() in self.junk_list, split_content))

        return ParsedHollyMessage(split_content, chat_id, sender, targeted)

class HollyMessage:
    """Represents a message for communication between HollyClient and HollyServer.

    Attributes:
        content: The content of the message.
        chat_id: Identifier of the chat the message belongs to.
        sender: Sender of the message.
    """

    def __init__(self, content=None, chat_id=None, sender="", json_data=None):
        if json_data:
            self.content = json_data['content']
            self.chat_id = json_data['chat_id']
            self.sender = json_data['sender']
        else:
            self.content = content
            self.chat_id = chat_id
            self.sender = sender

    def __str__(self):
        return str(self.to_dict())

    def __repr__(self) -> str:
        return self.__str__()

    def to_dict(self):
        """Converts the message to a dictionary.

        Returns:
            dict: A dictionary representation of the message.
        """
        return {
            'content': self.content,
            'chat_id': self.chat_id,
            'sender': self.sender
        }

    def serialize(self):
        """Serializes the message to JSON format.

        Returns:
            bytes: The serialized message in JSON format encoded in UTF-8.
        """
        return json.dumps(self.to_dict()).encode('utf-8')

    def parse(self, parser: HollyParser) -> ParsedHollyMessage:
        """Parses the message with the given HollyParser

        Returns:
            ParsedHollyMessage: The parsed message
        """
        return parser.parse(self.content, self.chat_id, self.sender)

class HollyClient:
    """Client for communicating with the HollyServer.

    Attributes:
        host (str): The host address of the server.
        port (int): The port number of the server.
        socket: The socket object for communication.
    """

    def __init__(self, host='localhost', port=8011):
        """
        Initializes the HollyClient instance and connects to the server.

        Args:
            host (str): The host address of the server. Default is 'localhost'.
            port (int): The port number of the server. Default is 8011.

        Raises:
            HollyError: If connection to the server fails.
        """
        self.host = host
        self.port = port
        try:
            self.socket =  socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.socket.connect((host, port))
        except ConnectionRefusedError as e:
            raise HollyError(f"Connection to server at {host}:{port} refused.") from e

    def recv(self) -> HollyMessage:
        """Receives a message from the server.

        Returns:
            HollyMessage: The received message.

        Raises:
            HollyError: If there's an issue receiving the message.
        """
        try:
            data = self.socket.recv(2048)
            json_data = json.loads(data.decode('utf-8'))
            return HollyMessage(json_data=json_data)
        except json.JSONDecodeError as e:
            raise HollyError("Failed to decode received message.") from e
        except Exception as e:
            raise HollyError(f"Failed to receive message: {e}") from e

    def send(self, msg: HollyMessage):
        """Sends a message to the server.

        Args:
            msg (HollyMessage): The message to be sent.

        Raises:
            HollyError: If there's an issue sending the message.
        """
        try:
            self.socket.send(msg.serialize())
        except Exception as e:
            raise HollyError(f"Failed to send message: {e}") from e

    def close(self):
        """Closes the connection to the server."""
        self.socket.close()

    def screenshot(self):
        """Command Holly core to take a screenshot"""
        self.send(HollyMessage("", "", "<screenshot>"))

    def html(self):
        """Command Holly core to dump the page HTML"""
        self.send(HollyMessage("", "", "<html>"))

    def restart(self):
        """Command Holly core to restart"""
        self.send(HollyMessage("", "", "<restart>"))

    def refresh(self):
        """Command Holly core to refresh the page"""
        self.send(HollyMessage("", "", "<refresh>"))
