window.fernspielctl = (() => {
  const uri = 'ws://127.0.0.1:38397'
  const protocol = 'fernspielctl'

  let socket

  return {
    reset () {
      return reconnect()
        .then(socket => {
          console.log('connection established')
          socket.send(JSON.stringify(
            {
              invoke: 'reset'
            }
          ))
        })
    },
    run (phonebook) {
      return reconnect()
        .then(socket => {
          console.log('connection established')
          const yaml = JSON.stringify(phonebook)
          const msg = JSON.stringify({
            invoke: 'run',
            with: yaml
          })
          socket.send(msg)
        })
    },
    runTestBook () {
      return this.run({
        initial: 'ok',
        states: {
          ok: {
            sounds: ['ok']
          }
        },
        sounds: {
          ok: {
            speech: 'deploy successful'
          }
        }
      })
    }
  }

  function reconnect () {
    return new Promise((resolve, reject) => {
      if (socket) {
        socket.close()
      }

      socket = new WebSocket(uri, protocol)
      socket.onopen = evt => {
        resolve(socket)
      }
      socket.onerror = err => {
        reject(err)
      }
    })
  }
})()
