# Talking to Fernspielapparat
This document gives you a whirlwind tour of _Fernspielapparat_,
an exciting way for you to tell an amazing story to a diverse
audience.

Now follow me, we are about to have some fun!

## Overview
As you probably know, _Fernspielapparat_ is the name of a wooden
box with a telephone inside it. The box controls the items inside
it with a hidden computer that operates on user input through an
analog dial and communicates with people in the box solely through
_speech_ and _lighting_.

We call the analog dial a sensor, giving _Fernspielapparat_ a
way to feel its surroundings. The speaker in the telephone
receiver and the set of lights in the box are called actuators,
and enable it to communicate with the world.

The interplay of sensors and actuators is controlled by the
stories and games people have written down in _phonebooks_,
small bundles of files understood by the hidden computer. If
you have a text editor on your computer, you have everything
you need to write your first phonebook. If you have a microphone
as well, even better.

## Starting out
Every _phonebook_ is a directory that contains at least a file
called _phonebook.yaml_ where the storytelling is laid out in
a format called YAML that is readable by computers but is also
relatively easy to read and write for humans. It's okay if you
never heard of YAML, you will learn as we move along. I'm sure
a smart person like you will get the hang of it quickly.

We start by creating a directory (a.k.a folder) and giving a
descriptive name. In it, we create an empty file with a text
editor like [Atom](https://atom.io/) and name it
`phonebook.yaml`. Any text editor is fine but office software
that typically deals with formatted text will probably put you
in a world of pain. Be sure to use the exact name `phonebook.yaml`
and double-check that the file-ending is `.yaml` and not something
like `.yaml.txt`.

## States
The story progresses by transitioning through _states_, each
defining their own speech or silence and light settings, which
are then realized by the actuators.

A simple phonebook can consist of only a single file. Check
out this phonebook that speaks the text "Hello, World!" over
and over:

    # Contents of phonebook.yaml
    states:
      hello:
        sounds:
          - hello
    sounds:
      hello:
        speech: Hello, World!
        loop: true

When we write `#`, we can make some notes that do not change
the meaning of the file. We then start off the real content
with a label `states:` that says we will now write down the
names of states, indented by two spaces, with a colon at the
end, in this case `hello:`. We then write down, each line
indented by four spaces, a category of things where this
state is _special_, in this case `sounds:`, that is, we let
the machine talk some text that we write down in the `sounds:`
section below.

You can write multiple lines of speech by starting with a `>`
and then continuing for a few lines, indented by two more
spaces. We can also emphasize by using `*emphasis asterisks*`
or add pauses by writing down one or more period characters:
`one. two..`. Adding `<ring>` into your text will let the
phone ring for dramatic purposes, at that point in the text.
Apart from speech, you can also control other aspects of the
machine like lighting, check it out:

    # Contents of phonebook.yaml
    states:
      countdown:
        sounds:
          - countdown
      destruction:
        sounds:
          - destruction
        lights:
          excitement: 100
    sounds:
      countdown:
        speech: >
          Three..
          Two..
          *One*..
      destruction:
        speech: Self-destruction initiated <ring>

You can see here that we now have two states. That's cool,
but the first one keeps repeating forever, how do we change
to self-destruct mode?

## Transitions
Easy, we define a transition from `countdown` to `destruction`.
A transition originates from one state and reacts to the value
of _sensors_, like the number typed into the dial. There is
also a sensor that reacts to the current state reaching the
end of text for the first time, it is called `end`, and is
exactly what we need for the example above:

    # Contents of phonebook.yaml
    states:
      countdown:
        sounds:
          - countdown
      destruction:
        sounds:
          - destruction
        lights:
          excitement: 100
    
    transitions:
      countdown:
        end: destruction

    # Sounds did not change
    sounds:
      countdown:
        speech: >
          Three..
          Two..
          *One*..
      destruction:
        speech: Self-destruction initiated <ring>

The `dial` sensor can have different transitions depending
on the number dialed. We can use it for this consentful
self-destruction version with an undo-feature:

    # Contents of phonebook.yaml
    states:
      announcement:
        speech: Dial zero to initiate self-destruction...
      countdown:
        sounds:
          - countdown
      destruction:
        sounds:
          - destruction
        lights:
          excitement: 100
    
    transitions:
      announcement:
        dial:
          0: countdown
      countdown:
        end: destruction
      destruction:
        dial:
          1: announcement

    sounds:
      announcement:
        speech: Dial zero to initiate self-destruction...
      countdown:
        speech: >
          Three..
          Two..
          *One*..
      destruction:
        speech: Self-destruction initiated <ring>

## Reading on
This is basically it, you know almost everything there is
to know about phone books. For a showcase of all possible
actuators and sensors, be sure to check out
[examples/phonebook.yaml](../examples/phonebook.yaml). If you
want to participate and write your own phonebook, get in
touch with us and we will gladly help you write what you
mean and get it installed on _Fernspielapparat_.

