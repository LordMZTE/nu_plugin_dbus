use nu_plugin::{
    serve_plugin, EngineInterface, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin,
};
use nu_protocol::{PluginExample, PluginSignature, Span, SyntaxShape, Type, Value};

mod client;
mod config;
mod convert;
mod dbus_type;
mod introspection;
mod pattern;

use client::*;
use config::*;

use crate::pattern::Pattern;

fn main() {
    serve_plugin(&NuPluginDbus, MsgPackSerializer)
}

/// The main plugin interface for nushell
struct NuPluginDbus;

impl Plugin for NuPluginDbus {
    fn signature(&self) -> Vec<PluginSignature> {
        macro_rules! str {
            ($s:expr) => {
                Value::string($s, Span::unknown())
            };
        }
        vec![
            PluginSignature::build("dbus")
                .dbus_command()
                .usage("Commands for interacting with D-Bus"),
            PluginSignature::build("dbus introspect")
                .dbus_command()
                .accepts_dbus_client_options()
                .accepts_timeout()
                .usage("Introspect a D-Bus object")
                .input_output_type(Type::Nothing, Type::Record(vec![]))
                .extra_usage("Returns information about available nodes, interfaces, methods, \
                    signals, and properties on the given object path")
                .required_named("dest", SyntaxShape::String,
                    "The name of the connection that owns the object",
                    None)
                .required("object", SyntaxShape::String,
                    "The path to the object to introspect")
                .plugin_examples(vec![
                    PluginExample {
                        example: "dbus introspect --dest=org.mpris.MediaPlayer2.spotify \
                            /org/mpris/MediaPlayer2 | explore".into(),
                        description: "Look at the MPRIS2 interfaces exposed by Spotify".into(),
                        result: None,
                    },
                    PluginExample {
                        example: "dbus introspect --dest=org.kde.plasmashell \
                            /org/kde/osdService | get interfaces | \
                            where name == org.kde.osdService | get 0.methods".into(),
                        description: "Get methods exposed by KDE Plasma's on-screen display \
                            service".into(),
                        result: None,
                    },
                    PluginExample {
                        example: "dbus introspect --dest=org.kde.KWin / | get children | \
                            select name".into(),
                        description: "List objects exposed by KWin".into(),
                        result: None,
                    },
                ]),
            PluginSignature::build("dbus call")
                .dbus_command()
                .accepts_dbus_client_options()
                .accepts_timeout()
                .usage("Call a method and get its response")
                .extra_usage("Returns an array if the method call returns more than one value.")
                .input_output_type(Type::Nothing, Type::Any)
                .named("signature", SyntaxShape::String,
                    "Signature of the arguments to send, in D-Bus format.\n    \
                     If not provided, they will be determined from introspection.\n    \
                     If --no-introspect is specified and this is not provided, they will \
                       be guessed (poorly)", None)
                .switch("no-flatten",
                    "Always return a list of all return values", None)
                .switch("no-introspect",
                    "Don't use introspection to determine the correct argument signature", None)
                .required_named("dest", SyntaxShape::String,
                    "The name of the connection to send the method to",
                    None)
                .required("object", SyntaxShape::String,
                    "The path to the object to call the method on")
                .required("interface", SyntaxShape::String,
                    "The name of the interface the method belongs to")
                .required("method", SyntaxShape::String,
                    "The name of the method to send")
                .rest("args", SyntaxShape::Any,
                    "Arguments to send with the method call")
                .plugin_examples(vec![
                    PluginExample {
                        example: "dbus call --dest=org.freedesktop.DBus \
                            /org/freedesktop/DBus org.freedesktop.DBus.Peer Ping".into(),
                        description: "Ping the D-Bus server itself".into(),
                        result: None
                    },
                    PluginExample {
                        example: "dbus call --dest=org.freedesktop.Notifications \
                            /org/freedesktop/Notifications org.freedesktop.Notifications \
                            Notify \"Floppy disks\" 0 \"media-floppy\" \"Rarely seen\" \
                            \"But sometimes still used\" [] {} 5000".into(),
                        description: "Show a notification on the desktop for 5 seconds".into(),
                        result: None
                    },
                ]),
            PluginSignature::build("dbus get")
                .dbus_command()
                .accepts_dbus_client_options()
                .accepts_timeout()
                .usage("Get a D-Bus property")
                .input_output_type(Type::Nothing, Type::Any)
                .required_named("dest", SyntaxShape::String,
                    "The name of the connection to read the property from",
                    None)
                .required("object", SyntaxShape::String,
                    "The path to the object to read the property from")
                .required("interface", SyntaxShape::String,
                    "The name of the interface the property belongs to")
                .required("property", SyntaxShape::String,
                    "The name of the property to read")
                .plugin_examples(vec![
                    PluginExample {
                        example: "dbus get --dest=org.mpris.MediaPlayer2.spotify \
                            /org/mpris/MediaPlayer2 \
                            org.mpris.MediaPlayer2.Player Metadata".into(),
                        description: "Get the currently playing song in Spotify".into(),
                        result: Some(Value::record(nu_protocol::record!(
                            "xesam:title" => str!("Birdie"),
                            "xesam:artist" => Value::list(vec![
                                str!("LOVE PSYCHEDELICO")
                            ], Span::unknown()),
                            "xesam:album" => str!("Love Your Love"),
                            "xesam:url" => str!("https://open.spotify.com/track/51748BvzeeMs4PIdPuyZmv"),
                        ), Span::unknown()))
                    },
                ]),
            PluginSignature::build("dbus get-all")
                .dbus_command()
                .accepts_dbus_client_options()
                .accepts_timeout()
                .usage("Get all D-Bus properties for the given object")
                .input_output_type(Type::Nothing, Type::Record(vec![]))
                .required_named("dest", SyntaxShape::String,
                    "The name of the connection to read the property from",
                    None)
                .required("object", SyntaxShape::String,
                    "The path to the object to read the property from")
                .required("interface", SyntaxShape::String,
                    "The name of the interface the property belongs to")
                .plugin_examples(vec![
                    PluginExample {
                        example: "dbus get-all --dest=org.mpris.MediaPlayer2.spotify \
                            /org/mpris/MediaPlayer2 \
                            org.mpris.MediaPlayer2.Player".into(),
                        description: "Get the current player state of Spotify".into(),
                        result: Some(Value::record(nu_protocol::record!(
                            "CanPlay" => Value::bool(true, Span::unknown()),
                            "Volume" => Value::float(0.43, Span::unknown()),
                            "PlaybackStatus" => str!("Paused"),
                        ), Span::unknown()))
                    },
                ]),
            PluginSignature::build("dbus set")
                .dbus_command()
                .accepts_dbus_client_options()
                .accepts_timeout()
                .usage("Set a D-Bus property")
                .input_output_type(Type::Nothing, Type::Nothing)
                .named("signature", SyntaxShape::String,
                    "Signature of the value to set, in D-Bus format.\n    \
                     If not provided, it will be determined from introspection.\n    \
                     If --no-introspect is specified and this is not provided, it will \
                       be guessed (poorly)", None)
                .required_named("dest", SyntaxShape::String,
                    "The name of the connection to write the property on",
                    None)
                .required("object", SyntaxShape::String,
                    "The path to the object to write the property on")
                .required("interface", SyntaxShape::String,
                    "The name of the interface the property belongs to")
                .required("property", SyntaxShape::String,
                    "The name of the property to write")
                .required("value", SyntaxShape::Any,
                    "The value to write to the property")
                .plugin_examples(vec![
                    PluginExample {
                        example: "dbus set --dest=org.mpris.MediaPlayer2.spotify \
                            /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player \
                            Volume 0.5".into(),
                        description: "Set the volume of Spotify to 50%".into(),
                        result: None,
                    },
                ]),
            PluginSignature::build("dbus list")
                .dbus_command()
                .accepts_dbus_client_options()
                .accepts_timeout()
                .usage("List all available connection names on the bus")
                .extra_usage("These can be used as arguments for --dest on any of the other commands.")
                .input_output_type(Type::Nothing, Type::List(Type::String.into()))
                .optional("pattern", SyntaxShape::String,
                    "An optional glob-like pattern to filter the result by")
                .plugin_examples(vec![
                    PluginExample {
                        example: "dbus list".into(),
                        description: "List all names available on the bus".into(),
                        result: None,
                    },
                    PluginExample {
                        example: "dbus list org.freedesktop.*".into(),
                        description: "List top-level freedesktop.org names on the bus \
                            (e.g. matches `org.freedesktop.PowerManagement`, \
                             but not `org.freedesktop.Management.Inhibit`)".into(),
                        result: Some(Value::list(vec![
                            str!("org.freedesktop.DBus"),
                            str!("org.freedesktop.Flatpak"),
                            str!("org.freedesktop.Notifications"),
                        ], Span::unknown())),
                    },
                    PluginExample {
                        example: "dbus list org.mpris.MediaPlayer2.**".into(),
                        description: "List all MPRIS2 media players on the bus".into(),
                        result: Some(Value::list(vec![
                            str!("org.mpris.MediaPlayer2.spotify"),
                            str!("org.mpris.MediaPlayer2.kdeconnect.mpris_000001"),
                        ], Span::unknown())),
                    },
                ])
        ]
    }

    fn run(
        &self,
        name: &str,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        match name {
            "dbus" => Err(LabeledError {
                label: "The `dbus` command requires a subcommand".into(),
                msg: "add --help to see subcommands".into(),
                span: Some(call.head),
            }),

            "dbus introspect" => self.introspect(call),
            "dbus call" => self.call(call),
            "dbus get" => self.get(call),
            "dbus get-all" => self.get_all(call),
            "dbus set" => self.set(call),
            "dbus list" => self.list(call),

            _ => Err(LabeledError {
                label: "Plugin invoked with unknown command name".into(),
                msg: "unknown command".into(),
                span: Some(call.head),
            }),
        }
    }
}

/// For conveniently adding the base options to a dbus command
trait DbusSignatureUtilExt {
    fn dbus_command(self) -> Self;
    fn accepts_dbus_client_options(self) -> Self;
    fn accepts_timeout(self) -> Self;
}

impl DbusSignatureUtilExt for PluginSignature {
    fn dbus_command(self) -> Self {
        self.search_terms(vec!["dbus".into()])
            .category(nu_protocol::Category::Platform)
    }

    fn accepts_dbus_client_options(self) -> Self {
        self.switch("session", "Send to the session message bus (default)", None)
            .switch("system", "Send to the system message bus", None)
            .switch(
                "started",
                "Send to the bus that started this process, if applicable",
                None,
            )
            .named(
                "bus",
                SyntaxShape::String,
                "Send to the bus server at the given address",
                None,
            )
            .named(
                "peer",
                SyntaxShape::String,
                "Send to a non-bus D-Bus server at the given address. \
                 Will not call the Hello method on initialization.",
                None,
            )
    }

    fn accepts_timeout(self) -> Self {
        self.named(
            "timeout",
            SyntaxShape::Duration,
            "How long to wait for a response",
            None,
        )
    }
}

impl NuPluginDbus {
    fn introspect(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        let config = DbusClientConfig::try_from(call)?;
        let dbus = DbusClient::new(config)?;
        let node = dbus.introspect(&call.get_flag("dest")?.unwrap(), &call.req(0)?)?;
        Ok(node.to_value(call.head))
    }

    fn call(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        let config = DbusClientConfig::try_from(call)?;
        let dbus = DbusClient::new(config)?;
        let values = dbus.call(
            &call.get_flag("dest")?.unwrap(),
            &call.req(0)?,
            &call.req(1)?,
            &call.req(2)?,
            call.get_flag("signature")?.as_ref(),
            &call.positional[3..],
        )?;

        let flatten = !call.get_flag::<bool>("no-flatten")?.unwrap_or(false);

        // Make the output easier to deal with by returning a list only if there are multiple return
        // values (not so common)
        match values.len() {
            0 if flatten => Ok(Value::nothing(call.head)),
            1 if flatten => Ok(values.into_iter().nth(0).unwrap()),
            _ => Ok(Value::list(values, call.head)),
        }
    }

    fn get(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        let config = DbusClientConfig::try_from(call)?;
        let dbus = DbusClient::new(config)?;
        dbus.get(
            &call.get_flag("dest")?.unwrap(),
            &call.req(0)?,
            &call.req(1)?,
            &call.req(2)?,
        )
    }

    fn get_all(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        let config = DbusClientConfig::try_from(call)?;
        let dbus = DbusClient::new(config)?;
        dbus.get_all(
            &call.get_flag("dest")?.unwrap(),
            &call.req(0)?,
            &call.req(1)?,
        )
    }

    fn set(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        let config = DbusClientConfig::try_from(call)?;
        let dbus = DbusClient::new(config)?;
        dbus.set(
            &call.get_flag("dest")?.unwrap(),
            &call.req(0)?,
            &call.req(1)?,
            &call.req(2)?,
            call.get_flag("signature")?.as_ref(),
            &call.req(3)?,
        )?;
        Ok(Value::nothing(call.head))
    }

    fn list(&self, call: &EvaluatedCall) -> Result<Value, LabeledError> {
        let config = DbusClientConfig::try_from(call)?;
        let dbus = DbusClient::new(config)?;
        let pattern = call
            .opt::<String>(0)?
            .map(|pat| Pattern::new(&pat, Some('.')));
        let result = dbus.list(pattern.as_ref())?;
        Ok(Value::list(
            result
                .into_iter()
                .map(|s| Value::string(s, call.head))
                .collect(),
            call.head,
        ))
    }
}
