# CUPS2MQTT

This is the repository for the CUPS2MQTT project. The goal of this project is to provide a way to monitor the status of a CUPS print server and its print queues/printers and send the status to an MQTT broker.

Additionally, this projects will provide the data to Home Assistant via the MQTT Discovery feature.

Status: Basic functionality in place, Docker image available on [Docker Hub](https://hub.docker.com/r/h3x4d3c1m4l/cups2mqtt).

## Current functionality

- [X] MQTT and CUPS connection details configurable
  - [X] Allows secure connection to MQTT broker and CUPS server
  - [X] Allows verification of TLS certificates through system CA store
  - [X] Allows TLS without verification of server certificate
- [X] Name, description, state and job count of printqueues are sent to MQTT broker
  - [ ] Supports job details
- [X] Home Assistant MQTT Discovery support
  - [X] Support for topology discovery
  - [ ] Online/Offline status (using LWT?)
- [ ] Control of print queues via MQTT
  - [ ] Pause/Resume print queues
  - [ ] Cancel print jobs
  - [ ] Restart print jobs
  - [ ] Add print jobs
- [ ] Ink or toner levels
- [ ] Error reporting through Sentry
- [ ] Handling disappeared print queues
- [ ] MQTT LWT support
- [X] Application packaging
  - [X] Docker image
  - [ ] Anything else, like Windows installer or perhaps a Homebrew package

## Troubleshooting

### Can't connect to a MQTT server by IP address with TLS enabled

This a limitation of [webpki](https://github.com/briansmith/webpki/issues/54), as such [rusttls](https://github.com/rustls/rustls/issues/184) can't create such connections at the moment, as such [rumqttc](https://docs.rs/rumqttc/latest/rumqttc/) can't create such connections.

Simply connect by hostname, if your server's hostname cannot be resolved using DNS, a workaround could be to add the hostname to your OS'es `hosts` file.
