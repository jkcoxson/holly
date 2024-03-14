"""Script to connect to Holly and then do silly things"""

import json
import socket
import time
import random

import ascii_dogs

def process_json_object(json_data):
    """Creates responses based on the message"""
    if 'good dog' in json_data['content'].lower():
        return {
            'chat_id': json_data['chat_id'],
            'content': 'I know I am',
            'sender': 'urmom'
        }
    if 'bad dog' in json_data['content'].lower():
        return {
            'chat_id': json_data['chat_id'],
            'content': 'no u',
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace('?', '') == 'who':
        return {
            'chat_id': json_data['chat_id'],
            'content': 'asked.',
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace('?', '') == 'holly':
        responses = ['yes?', 'wut', 'That\'s me!', 'I\'m Holly!', '*jumps up to lick your face*', 'hmmmm?', 'bork', 'Do you have a treat?']
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace('?', '') == 'thanks holly' or json_data['content'].lower().replace('?', '') == 'thank you holly':
        responses = ['yw', 'ofc', 'Yes, now give me a treat', 'Ok, can we go on a walk now?', '*jumps up to lick your face*', 'Anything for my favorite humans', 'bork', 'Just doing my job. Now give me a treat *begging eyes*']
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace(',', '') == 'holly shake':
        responses = ['Only if you have a treat', '*raises paw*', '*raises paw, but menacingly*', 'lol', '*shakes aggressively*', '*happily put paw in your hand*']
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace(',', '') == 'holly roll over':
        responses = ['Only if you have a treat', 'weeeeeeeeee', '*bark bark*', 'lol', '*rolls over*', 'I\'m too dizzy']
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace(',', '') == 'holly go to bed':
        responses = ['Only if you have a treat', 'One step ahead of you', 'zzzzzzzzzzzzzz', 'Don\'t need to ask me twice', 'k bet']
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace(',', '') == 'holly sit':
        responses = ['Only if you have a treat', 'lol no', ascii_dogs.SITTING]
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace(',', '').replace('!', '') == 'squirrel':
        responses = ['*BARK BARK BARK BARK BARK BARK BARK*', '*bolts away*', 'Ok, and?', 'Nice try, I\'m not running that far.', ascii_dogs.RUNNING]
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    if json_data['content'].lower().replace(',', '').replace('!', '') == 'cat':
        responses = ['*BARK BARK BARK BARK BARK BARK BARK*', '*bolts away*', 'Ok, and?', 'Nice try, I\'m not running that far.', 'My mortal enemy...', 'I must defend the humans!', ascii_dogs.RUNNING]
        return {
            'chat_id': json_data['chat_id'],
            'content': random.choice(responses),
            'sender': 'urmom'
        }
    return None

def main():
    """Main function"""
    host = 'localhost'
    port = 8011

    while True:
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.connect((host, port))
                while True:
                    data = s.recv(1024)
                    json_data = json.loads(data.decode('utf-8'))
                    response_data = process_json_object(json_data)

                    if response_data:
                        s.send(json.dumps(response_data).encode('utf-8'))
                    else:
                        print("No action needed for this JSON object.")

                    data = []

        except (socket.error, json.JSONDecodeError) as e:
            print(f"Error: {e}")

        print('Disconnected from Holly socket')
        time.sleep(30)

if __name__ == "__main__":
    main()
