use clap::{arg, command, value_parser};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Options {
    // Value in seconds use as a timeout for each Cocoa Groebner basis computation
    pub groebner_cocoa_timeout_seconds: u32,

    // Maximum number of variables inside the prohibition constraint before automatically
    //  timing out. This is needed because Cocoa hangs when multiplying the different terms
    //  in the prohibition polynomial (with large amounts of variables)
    pub max_vars_prohibition_polynomial_before_timeout: u32,

    // Boolean that specifies whether SVG diagrams should be drawn
    pub generate_svg_diagrams: bool,

    // True if only the last frame of the propagation process should be converted into an SVG,
    //  for better performance
    pub generate_only_last_propagation_svg: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            groebner_cocoa_timeout_seconds: 5,
            max_vars_prohibition_polynomial_before_timeout: 75,
            generate_svg_diagrams: false,
            generate_only_last_propagation_svg: false,
        }
    }
}

pub fn parse_command_line_arguments() -> (Option<PathBuf>, Options) {
    let matches = command!()
        .arg(
            arg!([folder] "Artifacts folder to operate on")
                .required_unless_present("usehardcodedpath")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(
                -t --timeout <TIMEOUT> "Sets a custom timeout for each Groebner basis computation in seconds"
            )
                // We don't have syntax yet for optional options, so manually calling `required`
                .required(false)
                .value_parser(value_parser!(u32))
                .default_value(OsString::from(Options::default().groebner_cocoa_timeout_seconds.to_string()))
        )
        .arg(
            arg!(
                -m --maxvars <MAXVARS> "Set a custom number of variables allowed inside a prohibition polynomial before timing-out"
            )
                .required(false)
                .value_parser(value_parser!(u32))
                .default_value(OsString::from(Options::default().max_vars_prohibition_polynomial_before_timeout.to_string()))
        )
        .arg(arg!(
            -s --svg "Turn SVG debug output"
        ))
        .arg(arg!(
            -p --propagationsvg "Generate all propagation steps SVG, not only one SVG after all propagations steps have been executed. Also enables SVG debug output"
        ))
        .arg(arg!(
            --usehardcodedpath "Use hard coded folder path from main.rs for debug purposes"
        ))
        .get_matches();

    let generate_only_last_propagation_svg = !matches.get_flag("propagationsvg");
    let generate_svg_diagrams = !generate_only_last_propagation_svg || matches.get_flag("svg");
    let groebner_cocoa_timeout_seconds = *matches.get_one::<u32>("timeout").unwrap();
    let max_vars_prohibition_polynomial_before_timeout =
        *matches.get_one::<u32>("maxvars").unwrap();

    let options = Options {
        groebner_cocoa_timeout_seconds,
        max_vars_prohibition_polynomial_before_timeout,
        generate_svg_diagrams,
        generate_only_last_propagation_svg,
    };

    let use_hardcoded_path = matches.get_flag("usehardcodedpath");

    let folder_path = if use_hardcoded_path {
        None
    } else {
        Some(matches.get_one::<PathBuf>("folder").unwrap().clone())
    };

    // println!("{:?}", folder_path);
    // println!("{:?}", options);

    (folder_path, options)
}
