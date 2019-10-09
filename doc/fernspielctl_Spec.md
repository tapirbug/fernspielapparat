# fernspielctl Remote Control Protocol 0.2.0
This document is a draft specification of the _fernspielctl_ protocol,
used by _fernspielapparat_ for remote control via WebSockets.

## Versioning
This document describes version 0.2.0 of the protocol. The version is
derived from the version of the reference implementation hosted on
[github.com/krachzack/fernspielapparat](https://github.com/krachzack/fernspielapparat).
Any breaking changes (from the perspective of the client) are indicated
by a bump in the major version. No such guarantees are made for server
implementations.

## Purpose
_fernspielctl_ enables _remote control_ of a running _fernspielapparat_
and the tracking of asynchronous events such as transitions between
states.

## Topology
The protocol is used for the communication between a server process,
which this specification refers to as the _fernspielapparat_ and a
number of clients connected to it. The two communicate through with
text messages on top of a reliable transport protocol. Framing of the
messages (indicate their start and end in a stream of text) must also
be handled by the transport protocol and is not part of this spec.

These clients are of various types, the most important being:
* phonebook editors that want to upload opened phonebooks and track the active state for testing,
* software and hardware components that want to send inputs to _fernspielapparat_ sensors to participate in gameplay or facilitate testing.

## Transport
The recommended way of transporting _fernspielctl_ messages is through
the WebSockets protocol, using text and close messages.

Though use of this communication medium is not a requirement of this spec,
the rest of this section is going to assume that WS are used and documents
the requirements only for this style of transport.

### WS Message Types
Binary messages MUST not be sent. Ping messages MAY be sent by clients and
MUST be responded by with a pong message from the server using the same
content that was sent by the client. Servers MAY attempt to send ping messages
to clients. Whether or not a client responds and what and the contents of the
response are not specified. The _fernspielapparat_ MUST NOT depend on clients
responding to the message and MUST NOT rely on a specific message content.
Both ends MAY orderly shut down connections with close messages.

### WS Protocol
During protocol negotiation, clients MUST communicate their intent to use
the protocol identified with the string `"fernspielctl"` for the server to
accept the connection upgrade.

### Default Port
_fernspielapparat_ binds to `0.0.0.0:38397` per default, but using this
port is not a strict requirement of this spec.

### HTTP Path
The path under which to host the WS server is not specified, but using
the root path is recommended.

## Message Flow
Remote control messages are one-off fire-and-forget text messages. The
text messages MUST conform to the YAML specification.

The YAML document MUST not be multi-part. A `---` MAY indicate the start
of the document. If it is used, it MUST be the only occurrence of the token
in the document.

The root MUST be an object holding at least an `"invoke"` key, with a command
string as value. If the command string implies it, a `"with"` key MUST be contained,
holding required arguments.

The key `"uuid"` on the root message MAY hold an optional unique identifier
of the message, with a UUIDv4 string as the value. This functionality
is not currently used in the protocol but will allow for responses to messages
in future versions of the spec.

The root object MUST contain no keys other than `"invoke"`, `"with"` and
`"uuid"`.

## Errors
This version of the protocol specification does not define a way to
communicate errors back to the API client. Any flow of errors in the other
direction is also not covered by this spec.

In case of an error or a malformed message, the implementation MAY attempt
to orderly shut down the connection to the client, but will not send any
error message with the WebSocket close message.

The ability to report errors is severely limited when the `"uuid"` is omitted
from a client request. Future versions of this spec might introduce a format
for errors that references the `"uuid"` values of one or more messages.

## Commands
Commands are requests from clients to a _fernspielapparat_ to perform an
operation. Some commands require a mandatory argument.

### Format
The request MUST be a YAML object holding at least the key `"invoke"` with
a value of string type. The value MUST be one of `"run"`, `"dial"` or `"reset"`.
Arguments MUST be specified under the `"with"` key of the object and MUST be
omitted when the command does not support arguments.

#### `invoke: "run"`
Communicates a request to stop the currently running phonebook to instead
start from the initial state of a phonebook that is specified as the command
argument.

`"with"` MUST contain a valid phonebook conforming to the phonebook spec of
the same version as the protocol version used.

#### `invoke: "dial"`
Sends input to the _fernspielapparat_ as if it were dialed on the device
itself.

The `"with"` key MUST have a value of type string that conforms to the
following regular expression `[0-9hp]+`. The inputs are processed in the
_fernspielapparat_ in the order they were specified in the string, with
the following interpretation:

`0`-`9`: Dial the number denoted by the digit.

`h`: Behave as if the phone just hung up.

`p`: Behave as if the phone was just picked up.

#### `invoke: "reset"`
Request that the currently active phonebooks restarts from the initial state,
as if it were freshly opened.

The `"with"` key MUST be omitted.

## Events
Events are broadcasted from the _fernspielapparat_ implementation to all
connected WebSocket clients to inform them of events regarding the execution
of the current phonebook. It is irrelevant if the phonebook has been specified
at startup or was remotely set by a client, all connected clients will receive
the events.

Events MUST be YAML objects holding at least a the key `"type"` mapped to
one of the strings `"start"`, `"transition"` or `"finish"`. Any other properties
provide additional context, according to the event type.

### `type: "start"`
Communicates that a phonebook has just been loaded and starts from an initial
state that is specified with the message. Also sent in the case of resets.

MUST also have the key `"initial"` on the root object, mapped to an object
only holding a key `"id"`, mapped to the unique identifier of the initial
state of the current phonebook.

Example:

    type: start
    initial:
      id: initial


### `type: "transition"`
Indicates that a transition happened from one state to another in response
the an event.

The event object MUST define a key `"reason"` on the root object that holds
and object with additional information on the event that led to the transition.
The key MAY be one of `"timeout"` or `"dial"` and be mapped to either a number
of a string with additional information. Any other string key is also permitted,
communicating some kind of event, with unspecified value.

MUST define `"from"` and `"to"` on the root object, mapped to an object
only holding a key `"id"`, mapped to the unique identifier of originating
state and the target state, respectively.

Example:

    type: transition
    reason:
      timeout: 1.0
    from:
      id: initial
    to:
      id: terminal"

### `type: "finish"`
Sent when a terminal state has been reached on the currently running phonebook.

MUST also have the key `"terminal"` on the root object, mapped to an object
only holding a key `"id"`, mapped to the unique identifier of the state.

Example:

    type: finish
    terminal:
      id: terminal

