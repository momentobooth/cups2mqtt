# CUPS2MQTT

This is the repository for the CUPS2MQTT project. The goal of this project is to provide a way to monitor the status of a CUPS print server and its print queues/printers and send the status to an MQTT broker.

Additionally, this projects will provide the data to Home Assistant via the MQTT Discovery feature.

Status: WIP (not ready for use)

## Troubleshooting

### Can't connect to a MQTT server by IP address with TLS enabled

This a limitation of [webpki](https://github.com/briansmith/webpki/issues/54), as such [rusttls](https://github.com/rustls/rustls/issues/184) can't create such connections at the moment, as such [rumqttc](https://docs.rs/rumqttc/latest/rumqttc/) can't create such connections.

Simply connect by hostname, if your server's hostname cannot be resolved using DNS, a workaround could be to add the hostname to your OS'es `hosts` file.
