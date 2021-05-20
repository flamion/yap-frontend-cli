use cursive::Cursive;
use cursive::views::{Dialog, EditView, LinearLayout, TextView, Checkbox};
use cursive::view::{Nameable, Resizable};
use regex::Regex;
use std::ops::Not;
use reqwest::blocking;
use reqwest::StatusCode;
use std::io::Write;
use std::fs;
use std::fs::{File, OpenOptions};
use serde_json;
use std::path::{Path, PathBuf};
use serde::Serialize;
use serde::Deserialize;
use xdg;
//use std::thread;
//use std::sync::mpsc;


//TODO get rid of the token option
//TODO put the request parts in its own call_backend function
//TODO rewrite login so it takes email and password as arguments

//TOKEN_FILE name
static TOKEN_FILE: &'static str = "token.json";


struct GlobalData {
    http_client: blocking::Client,
    token: Option<String>,
    config_home: xdg::BaseDirectories
}

#[derive(Serialize, Deserialize)]
struct TokenFile {
    user_mail: String,
    token: String,
}

enum RegisterInvalid {
    InvalidUsername,
    InvalidEmail,
    InvalidPassword,
}

enum TokenLoadError {
    TokenExpired,
    FileNotFound,
    FileNotReadable,
}

//TODO implement the call_backend() function
/*enum RequestError {
    StatusError,
    RequestFailed,
}*/

fn exit(root: &mut Cursive) {
    root.quit();
}

fn login_page(root: &mut Cursive) {
    root.pop_layer();
    root.add_layer(Dialog::new()
        .title("Login - YAP")
        .content(
        LinearLayout::vertical()
            .child(
                TextView::new("\nemail:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .with_name("EMAIL_LOGIN")
                    .fixed_width(34)
            )
            .child(
                TextView::new("\npassword:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .secret()
                    .with_name("PASSWORD_LOGIN")
                    .fixed_width(34)
            )
            .child(
                TextView::new("\n")
            )
            .child(
            LinearLayout::horizontal()
                .child(
                    Checkbox::new()
                        .on_change(|root, state|
                            if let Some(mut view) =
                            root.find_name::<EditView>("PASSWORD_LOGIN") {
                                view.set_secret(state.not());
                            }
                        )
                )
                .child(
                    TextView::new(" Show password")
                        .fixed_width(16)
                )
                .child(
                    Checkbox::new()
                        .with_name("REMEMBER_ME_LOGIN")
                )
                .child(
                    TextView::new(" Remember Me")
                )
            )
        )
        .button("Back", |root| welcome_page(root))
        .button("Login", login)
    );
}

fn login(root: &mut Cursive) {

    let email = root.find_name::<EditView>("EMAIL_LOGIN")
        .unwrap_or_else(
            || root.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name"))
        .get_content();

    let password = root.find_name::<EditView>("PASSWORD_LOGIN")
        .unwrap_or_else(
            || root.find_name::<EditView>("PASSWORD_REGISTER")
                .expect("couldn't find view by name"))
        .get_content();

    //let config_dir = &root.user_data::<GlobalData>()
    //    .unwrap()
    //    .config_home;

    //Get HTTP client if it exists else create one and store it for later use
    /*let http_client = root.take_user_data::<GlobalData>().unwrap_or(
        GlobalData {
            http_client: blocking::Client::new(),
            //token: "".to_string(),
            token: None,
        }).http_client;*/


    //file.unwrap().write_all(password.as_bytes()).unwrap();
    //let mut filee = File::create(root.user_data::<GlobalData>().unwrap().config_home.find_data_file(TOKEN_FILE).expect("file not found")).expect("file wasn't created");
    //let mut file = File::create("reached");

    //Send request to backend to obtain a token
    match root.user_data::<GlobalData>().expect("no user data set").http_client.post("https://backend.yap.dragoncave.dev/security/token")
        .header("content-type", "application/json")
        .body(format!(
            "{{\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            email,
            password
        ))
        .send() {

        Ok(request) => { // <- If the status code is an Error it will still return an Ok()
            if request.status().is_success() {
                remove_file(root, TOKEN_FILE);

                root.with_user_data(|data: &mut GlobalData | {
                    data.token = Some(request.text().unwrap());
                });

                //Write the token to a file if REMEMBER_ME is checked
                if let Some(state) = root.find_name::<Checkbox>(
                    "REMEMBER_ME_LOGIN"
                ) {
                    if state.is_checked().eq(&true) {

                        create_file(root, TOKEN_FILE);

                        if let Ok(mut file) = get_file(root, TOKEN_FILE) {
                            file.write_all(
                                serde_json::to_string_pretty(
                                    &TokenFile {
                                        user_mail: email.to_string(),
                                        token: root.user_data::<GlobalData>()
                                            .expect("no user data set")
                                            .token
                                            .as_ref()
                                            .unwrap()
                                            .to_string(),
                                    })
                                    .unwrap()
                                    .as_bytes()
                            ).expect("couldn't write to token file");
                        } else {
                            notify_popup(root, "Remember Me", "Remember Me didn't work :(")
                        }
                    }
                } else {
                    notify_popup(root, "No success!", "file not created");
                }
                main_screen(root);
            } else {
                if request.status() == StatusCode::FORBIDDEN {
                    notify_popup(
                        root,
                        "Wrong credentials!",
                        "Either your mail or password is wrong."
                    );
                } else {
                    notify_popup(
                        root,
                        "Request failed.",
                        &*format!(
                            "Request failed with code: {}",
                            request.status().as_str()
                        )
                    )
                }
            }
        },
        Err(error) => {
            if let Some(status) = error.status() {
                notify_popup(root, "Request failed.",
                             &*format!("Request failed with code: {}", status))
            } else {
                notify_popup(root, "Request failed.", "Reason: Unknown");
            }
        },
    }
}


fn main_screen(root: &mut Cursive) {
    root.pop_layer();
    notify_popup(root, "eeeeeemptyness", "hmm, doesn't seem to be ready yet.")
}

fn register_page(root: &mut Cursive) {
    root.pop_layer();
    root.add_layer(Dialog::new()
        .title("Register - YAP")
        .content(
        LinearLayout::vertical()
            .child(
                TextView::new("\nusername:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .with_name("USERNAME_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\nemail:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .with_name("EMAIL_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\npassword:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .secret()
                    .with_name("PASSWORD_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\nretype password:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .secret()
                    .with_name("PASSWORD_CHECK_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\n")
            )
            .child(
            LinearLayout::horizontal()
                .child(
                Checkbox::new()
                    .on_change(|root, state|
                        if let Some(mut view) =
                        root.find_name::<EditView>("PASSWORD_REGISTER") {
                            if let Some(mut check_view) =
                            root.find_name::<EditView>("PASSWORD_CHECK_REGISTER") {

                                view.set_secret(state.not());
                                check_view.set_secret(state.not());
                            }
                        }
                    )
                )
                .child(
                    TextView::new(" Show password")
                )
            )
        )
        .button("Back", |root| welcome_page(root))
        .button("Register and login", |root| {
                match check_register(root) {
                    Ok(_) => register(root),
                    Err(RegisterInvalid::InvalidUsername) =>
                        notify_popup(root, "credentials not valid",
                                     "error: username not valid"),
                    Err(RegisterInvalid::InvalidEmail) =>
                        notify_popup(root, "credentials not valid",
                                     "error: email not valid"),
                    Err(RegisterInvalid::InvalidPassword) =>
                        notify_popup(root, "credentials not valid",
                                     "error: password not valid"),
            }
        })
    );
}

fn notify_popup(root: &mut Cursive, title: &str, message: &str) {
    root.add_layer(
        Dialog::text(message)
            .title(title)
            .dismiss_button("Ok")
    );
}

/*fn call_backend(root: &mut Cursive, end_point: &str, headers: reqwest::header::HeaderMap, body: &str) -> Result<String, RequestError> {
    let &mut http_client = &root.user_data::<User>().unwrap().http_client;

    let mut request = http_client.post(format!("https://backend.yap.dragoncave.dev/{}", end_point))
        .body(body)
        .headers(headers);

    match request.send() {
        Ok(response) => {
            if response.status().is_success() {
                return Ok(response.text().unwrap());
            } else {
                return Err(RequestError::StatusError);
            }
        },
        Err(error) => {
            return Err(RequestError::RequestFailed);
        }
    };
}*/

fn check_register(root: &mut Cursive) -> Result<(), RegisterInvalid> {
    let username = root.find_name::<EditView>("USERNAME_REGISTER")
        .expect("couldn't find view by name");

    let email = root.find_name::<EditView>("EMAIL_REGISTER")
        .expect("couldn't find view by name");

    let password = root.find_name::<EditView>("PASSWORD_REGISTER")
        .expect("couldn't find view by name");

    let password_check = root.find_name::<EditView>("PASSWORD_CHECK_REGISTER")
        .expect("couldn't find view by name");


    if username.get_content().as_str().eq("") ||
        username.get_content().len() > 32 {

        return Err(RegisterInvalid::InvalidUsername);
    }

    if email.get_content().as_str().eq("") ||
        Regex::new(/*<editor-fold desc="email regEx">*/"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|\"(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21\\x23-\\x5b\\x5d-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])*\")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21-\\x5a\\x53-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])+)\\])"
                   /*</editor-fold>*/).unwrap().is_match(
            email.get_content().as_str()
        ).not() {

        return Err(RegisterInvalid::InvalidEmail);
    }

    if password.get_content().len() < 10 ||
        password.get_content().eq(&password_check.get_content()).not() ||
        password.get_content().len() > 1024 {

        return Err(RegisterInvalid::InvalidPassword);
    }
    return Ok(());
}

fn register(root: &mut Cursive) {
    match root.user_data::<GlobalData>().expect("no user data set").http_client.post("https://backend.yap.dragoncave.dev/user")
        .header("content-type", "application/json")
        .body(format!(
            "{{\"username\":\"{}\",\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            root.find_name::<EditView>("USERNAME_REGISTER")
                .expect("couldn't find view by name").
                get_content(),
            root.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name")
                .get_content(),
            root.find_name::<EditView>("PASSWORD_REGISTER")
                .expect("couldn't find view by name")
                .get_content()
        ))
        .send() {
        Ok(_) => {
            notify_popup(root, "Success!", "Successfully created user");
        },
        Err(error) => {
            if let Some(status) = error.status() {
                notify_popup(root, "Request failed.",
                             &*format!("Request failed with code: {}", status))
            } else {
                notify_popup(root, "Request failed.", "Reason: Unknown");
            }
        },
    }
    root.pop_layer();
    login(root);
}

fn get_path(root: &mut Cursive, file: &str) -> Result<PathBuf, std::io::ErrorKind> {
    if let Some(path) = &root.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .find_config_file(file) {
        return Ok(path.clone());
    } else {
        return Err(std::io::ErrorKind::NotFound);
    }
}

fn get_file(root: &mut Cursive, file: &str) -> Result<File, std::io::ErrorKind> {
    if let Ok(path) = get_path(root, file) {
        return Ok(OpenOptions::new()
            .write(true)
            .open(path)
            .expect("file couldn't be opened"));
    } else {
        return Err(std::io::ErrorKind::NotFound);
    }
}

//TODO add Result<(), Error>
fn remove_file(root: &mut Cursive, file: &str) {
    if let Some(file_path) = &root.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .find_config_file(file) {

        fs::remove_file(file_path)
            .expect(
                &*format!(
                    "couldn't remove {} file",
                    file_path.to_str().unwrap()
                )
            );
    }
}

//TODO add Result<(), Error>
fn create_file(root: &mut Cursive, file: &str) {
    match root.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .place_config_file(file) {
        Ok(file_path) => if Path::exists(file_path.as_ref()).not() {
            File::create(file_path).expect("couldn't create file");
        }
        Err(error) => panic!("{}", error.to_string()),
    }
}

fn welcome_page(root: &mut Cursive) {
    root.pop_layer();
    root.add_layer(Dialog::text(
        "Welcome to YAP!\nPress <Login> if you already have an account, \
         else consider creating one by pressing <Register>"
    )
        .title("Login - YAP")
        .button("Quit", |root| root.quit())
        .button("Login", login_page)
        .button("Register", register_page));
}

//TODO use the client from user_data
fn check_token(root: &mut Cursive, token: &str) -> bool {
    if let Ok(response) = root.user_data::<GlobalData>().expect("no user data set").http_client.get("https://backend.yap.dragoncave.dev/security/token/checkValid")
        .header("token", token)
        .send() {
        if let Ok(status) = response.text().unwrap().parse::<bool>() {
            return status;
        } else {
            return false;
        }
    } else {
        return false;
    }
}

//TODO change error type to a generic one?
fn load_token(root: &mut Cursive) -> Result<TokenFile, TokenLoadError> {
    if let Ok(path) = get_path(root, TOKEN_FILE) {
        if let Ok(token_content) = fs::read_to_string(path) {
            if let Ok(token_struct) = serde_json::from_str::<TokenFile>(&*token_content) {
                if check_token(root, &token_struct.token) {
                    return Ok(token_struct);
                } else {
                    return Err(TokenLoadError::TokenExpired);
                }
            } else {
                return Err(TokenLoadError::FileNotReadable);
            }
        } else {
            return Err(TokenLoadError::FileNotReadable);
        }
    } else {
        return Err(TokenLoadError::FileNotFound);
    }
}


fn main() {
    //initialize objects
    let mut root = cursive::default();

    //bind exit to 'q' to be able to exit at any time
    root.add_global_callback('q', exit);

    root.add_global_callback('\\', Cursive::toggle_debug_console);

    root.set_user_data(GlobalData {
        token: None,
        http_client: blocking::Client::new(),
        config_home: xdg::BaseDirectories::with_prefix("yap").unwrap(),
    });

    //load theme file if present
    if let Ok(file) = get_path(&mut root, "theme.toml") {
        root.load_theme_file(file).unwrap();
    }

    //root.user_data::<GlobalData>().unwrap().config_home.place_data_file(TOKEN_FILE).expect("token file not placed");
    //root.with_user_data(|data: &mut GlobalData| data.config_home.place_data_file(TOKEN_FILE).expect("couldn't place token file"));

    //display the welcome page
    //TODO integrate this mess into the login function, which should by then use the credentials as parameters
    if let Ok(token_comb) = load_token(&mut root) {
        root.add_layer(Dialog::text(
            format!("Is {} you?", token_comb.user_mail))
            .button("yes", move |mut root| {

                //let http_client = blocking::Client::new();

                /*root.set_user_data(
                    GlobalData {
                        token: Some(token_comb.token.clone()),
                        http_client,

                    }
                );*/

                root.with_user_data(|data: &mut GlobalData| {
                    data.token = Some(token_comb.token.clone());
                });

                main_screen(&mut root);
            })
            .button("no", |root| {
                remove_file(root, TOKEN_FILE);
                login_page(root);
            })
        );
    } else {
        welcome_page(&mut root);
    }

    //start the event loop
    root.run();
}
