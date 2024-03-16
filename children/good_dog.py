"""Script to connect to Holly and then do silly things"""

import random
import time
import ascii_dogs
import holly
import thoughts

def process_message(msg: holly.ParsedHollyMessage):
    """Creates responses based on the message"""
    print(msg)

    if msg.loose_match("good dog"):
        return "I know I am"

    if msg.loose_match("bad dog"):
        return "no u"

    if msg.loose_match("can I get amen"):
        return "amen"

    if msg.match("who"):
        return "asked."

    if msg.match([]) and msg.is_targeted():
        responses = ['yes?', 'wut', 'That\'s me!', 'I\'m Holly!', '*jumps up to lick your face*', 'hmmmm?', 'bork', 'Do you have a treat?']
        return random.choice(responses)

    if msg.match("thanks holly") or msg.match("thank you holly"):
        responses = ['yw', 'ofc', 'Yes, now give me a treat', 'Ok, can we go on a walk now?', '*jumps up to lick your face*', 'Anything for my favorite humans', 'bork', 'Just doing my job. Now give me a treat *begging eyes*']
        return random.choice(responses)

    if msg.is_targeted() and msg.match("shake"):
        responses = ['Only if you have a treat', '*raises paw*', '*raises paw, but menacingly*', 'lol', '*shakes aggressively*', '*happily put paw in your hand*']
        return random.choice(responses)

    if msg.is_targeted() and msg.match("roll over"):
        responses = ['Only if you have a treat', 'weeeeeeeeee', '*bark bark*', 'lol', '*rolls over*', 'I\'m too dizzy']
        return random.choice(responses)

    if msg.is_targeted() and msg.match("go to bed"):
        responses = ['Only if you have a treat', 'One step ahead of you', 'zzzzzzzzzzzzzz', 'Don\'t need to ask me twice', 'k bet']
        return random.choice(responses)

    if msg.is_targeted() and msg.match("sit"):
        responses = ['Only if you have a treat', 'lol no', ascii_dogs.SITTING]
        return random.choice(responses)

    if msg.match("squirrel"):
        responses = ['*BARK BARK BARK BARK BARK BARK BARK*', '*bolts away*', 'Ok, and?', 'Nice try, I\'m not running that far.', ascii_dogs.RUNNING]
        return random.choice(responses)

    if msg.match("cat"):
        responses = ['*BARK BARK BARK BARK BARK BARK BARK*', '*bolts away*', 'Ok, and?', 'Nice try, I\'m not running that far.', 'My mortal enemy...', 'I must defend the humans!', ascii_dogs.RUNNING]
        return random.choice(responses)

    if msg.is_targeted() and (msg.match("what you thinking") or msg.match("say something")):
        return random.choice(thoughts.THOUGHTS)

    if msg.is_targeted() and msg.match("speak"):
        return random.choice(thoughts.SPEAK)
    return None

def main():
    """Main function"""

    parser = holly.HollyParser()

    while True:
        try:
            client = holly.HollyClient()
            print('Connected to Holly')
            while True:
                raw_msg = client.recv()
                print(raw_msg)
                ret = process_message(raw_msg.parse(parser))
                if ret:
                    client.send(holly.HollyMessage(content=ret, chat_id=raw_msg.chat_id, sender=''))

        except holly.HollyError as e:
            print(f"Error: {e}")

        print('Disconnected from Holly socket')
        time.sleep(30)

if __name__ == "__main__":
    main()
