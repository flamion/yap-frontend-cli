use cursive::Cursive;
use cursive::views::{Dialog, EditView, LinearLayout, TextView, Checkbox};
use cursive::view::{Nameable, Resizable};
use regex::Regex;
use std::ops::Not;
use reqwest::blocking;
use reqwest::StatusCode;
use std::io::Write;
use std::fs;
use std::fs::File;
use serde_json;
use std::path::Path;
use serde::Serialize;
use serde::Deserialize;
//use std::thread;
//use std::sync::mpsc;

//TODO rewrite login so it takes email and password as arguments

//TOKEN_FILE location
static TOKEN_FILE: &'static str = "token.json";

struct User {
    http_client: blocking::Client,
    token: Option<String>,
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
            /*)
            .child(
            LinearLayout::horizontal()
                .child(
                    Checkbox::new()
                        .on_change(|root, state| {

                        })
                )
                .child(
                    TextView::new(" Remember Me")
                )*/
            )
        )
        .button("Back", |root| welcome_page(root))
        .button("Login", login)
    );
}

fn login(root: &mut Cursive) {

    //Get HTTP client if it exists else create one and store it for later use
    let http_client=  root.take_user_data::<User>().unwrap_or(
        User {
            http_client: blocking::Client::new(),
            //token: "".to_string(),
            token: None,
        }).http_client;

    /*let email = root.find_name::<EditView>("EMAIL_LOGIN")
        .unwrap_or(
            root.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name")
        )
        .get_content();*/

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

    //let mut file = File::create("reached");
    //file.unwrap().write_all(password.as_bytes()).unwrap();

    //Send request to backend to obtain a token
    match http_client.post("https://backend.yap.dragoncave.dev/security/token")
        .header("content-type", "application/json")
        .body(format!(
            "{{\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            email,
            password
        ))
        .send() { // <- Some error isn't properly caught, to try type invalid info into login form and observe that is still executes the successful branch

        Ok(request) => {
            //let mut file = File::create("reached");
            //file.unwrap().write_all(request.status().as_str().as_bytes()).unwrap();

            if request.status().is_success() {
                if let Ok(_) = fs::remove_file(TOKEN_FILE) {};
                //Store the client and token
                root.set_user_data(User {
                    http_client,
                    token: Some(request.text()
                        .expect("request didn't return a token")),
                });

                //Write the token to a file if REMEMBER_ME is checked
                if let Some(state) = root.find_name::<Checkbox>(
                    "REMEMBER_ME_LOGIN"
                ) {
                    if state.is_checked().eq(&true) {
                        if Path::exists(TOKEN_FILE.as_ref()).not() {
                            fs::remove_file(TOKEN_FILE).unwrap();
                        }

                        let mut file = File::create(TOKEN_FILE).expect(
                            "TOKEN_FILE couldn't be created"
                        );
                        file.write_all(
                            serde_json::to_string_pretty(
                                &TokenFile {
                                    user_mail: root.find_name::<EditView>("EMAIL_LOGIN")
                                        .expect("couldn't find view by name")
                                        .get_content()
                                        .to_string(),
                                    token: root.user_data::<User>()
                                        .expect("no user data set")
                                        .token
                                        .as_ref()
                                        .unwrap()
                                        .to_string(),
                                })
                                .unwrap()
                                .as_bytes()
                        ).expect("couldn't write to token file");
                    }
                } else {
                    notify_popup(root, "No success!", "file not created");
                }
                main_screen(root);
            } else {
                if request.status() == StatusCode::UNAUTHORIZED {
                    notify_popup(
                        root,
                        "Wrong credentials!",
                        "Ether your mail or password is wrong."
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

/*fn remember_me_login(root: &mut Cursive) {
    if Path::exists(TOKEN_FILE.as_ref()) { //error is between here ----------------------------------------------------
        let mut json = String::new();
        File::open(TOKEN_FILE)
            .unwrap()
            .read_to_string(&mut json);
        let json = serde_json::from_str::<TokenFile>(&*json).unwrap();

        let token = json.token;
        //let user_mail = json.user_mail;

        if let Ok(status) = root.user_data::<User>()
            .unwrap()
            .http_client
            .get("https://backend.yap.dragoncave.dev/security/token/checkValid")
            .header("Token", token.clone())
            .send() {
            if status.text().unwrap().parse::<bool>().unwrap() {
                root.with_user_data::<_, User, _>(|user| user.token = token.clone());
                notify_popup(root, "Success!", "Could import token, noice!"); // <- should eventually be replaced with main_screen(root)
            } else {
                fs::remove_file(TOKEN_FILE);
                root.add_layer(
                    Dialog::text(
                        "your token is probably invalid, you should try to reauthenticate"
                    )
                        .title("¯\\_(ツ)_/¯")
                        .button("reauthenticate", |root| {
                            root.pop_layer();
                            root.pop_layer();
                            login_page(root);
                        })
                );
            }
        }
    }
}*/

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

    /*if let Some(view) = root.find_name::<EditView>("USERNAME_REGISTER") {
        if view.get_content().as_str().eq("") || view.get_content().len() > 32 {
            return Err(RegisterInvalid::InvalidUsername);
        }
    } else {
        panic!("couldn't find view by name");
    }


    if let Some(view) = root.find_name::<EditView>("EMAIL_REGISTER") {
        if view.get_content().as_str().eq("") ||
            Regex::new(/*<editor-fold desc="email regEx">*/"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|\"(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21\\x23-\\x5b\\x5d-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])*\")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21-\\x5a\\x53-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])+)\\])"
                       /*</editor-fold>*/).unwrap().is_match(view.get_content().as_str()).not() {
            return Err(RegisterInvalid::InvalidEmail);
        }
    } else {
        panic!("Couldn't find view by name");
    }


    if let Some(view) = root.find_name::<EditView>("PASSWORD_REGISTER") {
        if let Some(check_view) = root.find_name::<EditView>("PASSWORD_CHECK_REGISTER") {
            if view.get_content().len() < 10 ||
                view.get_content().eq(&check_view.get_content()).not() ||
                view.get_content().len() > 1024 {
                return Err(RegisterInvalid::InvalidPassword);
            }
        }
    } else {
        panic!("Couldn't find view by name");
    }*/

    return Ok(());
}

fn register(root: &mut Cursive) {
    /*let username = root.find_name::<EditView>("USERNAME")
        .unwrap().
        get_content();
    let email = root.find_name::<EditView>("EMAIL")
        .unwrap()
        .get_content();
    let password = root.find_name::<EditView>("PASSWORD")
        .unwrap()
        .get_content();

    call_api(root, "add_user", format!(
        "{{\"username\":\"{}\",\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
        username,
        email,
        password
    ));*/

    let http_client = blocking::Client::new();

    match http_client.post("https://backend.yap.dragoncave.dev/user")
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
            root.set_user_data(User {
                http_client,
                //token: "".to_string(),
                token: None,
            });
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
    login(root);
}

/*fn api_caller(root: &mut Cursive, endpoint: &str) {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        //let response = client.post("https://backend.yap.dragoncave.dev/user")
        let response = client.post("http://slowwly.robertomurray.co.uk/delay/2000/url/http://www.google.co.uk")
            .header("Content-Type", "application/json")
            //.body("{\"username\":\"username-dawdwa\",\"emailAddress\":\"emaildawa@wdaad.aa\",\"password\":\"passworddawdawd\"}")
            .send().unwrap();

        &tx.send(response);
    });


    loop {
        match rx.try_recv() {
            Ok(message) => {
                notify_popup(root, "worked!", &*format!("token: {}", message.text().unwrap()));
            },
            Err(error) => match error {
                mpsc::TryRecvError::Empty => (),
                mpsc::TryRecvError::Disconnected => break,
            }
        }
    };
}*/

/*fn call_api(root: &mut Cursive, action: &str, body: String) {

    let data: &mut Data = root.user_data::<Data>().unwrap();

    let http_client = &data.http_client;

    match action {
        "add_user" => {
            match http_client.post("https://backend.yap.dragoncave.dev/user")
                .header("Content-Type", "application/json")shadow = false
                .body(body)
                .send() {
                Ok(request) => {
                    data.user_id = request.text()
                        .unwrap()
                        .parse::<i64>()
                        .unwrap();

                    root.set_user_data(&data);
                    notify_popup(root, "Success!", "Successfully created user")
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
        },
        "delete_user" => {

        },
        "get_entries" => {

        },
        "get_user" => {

        },
        "modify_user" => {

        },
        invalid_action => panic!("invalid action: {}", invalid_action)
    }
}*/

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

fn check_token(token: &str) -> bool {
    let client = reqwest::blocking::Client::new();

    if let Ok(response) = client.get("https://backend.yap.dragoncave.dev/security/token/checkValid")
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

fn load_token() -> Result<TokenFile, TokenLoadError> {
    if Path::new(TOKEN_FILE).exists().not() {
        return Err(TokenLoadError::FileNotFound);
    }
    if let Ok(token_content) = fs::read_to_string(TOKEN_FILE) {
        if let Ok(token_struct) = serde_json::from_str::<TokenFile>(&*token_content) {
            if check_token(&token_struct.token) {
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
}


fn main() {
    let theme_path = include_str!("../theme.toml");

    //initialize objects
    let mut root = cursive::default();

    //load theme file if present
    if let Ok(_) = root.load_toml(theme_path) {}

    //bind exit to 'q' to be able to exit at any time
    root.add_global_callback('q', exit);

    root.add_global_callback('\\', Cursive::toggle_debug_console);



    //display the welcome page
    if let Ok(token_comb) = load_token() {
        root.add_layer(Dialog::text(
            format!("Is {} you?", token_comb.user_mail))
            .button("yes", move |mut root| {

                let http_client = blocking::Client::new();

                root.set_user_data(
                    User {
                        token: Some(token_comb.token.clone()),
                        http_client
                    }
                );
                main_screen(&mut root);
            })
            .button("no", |root| {
                fs::remove_file(TOKEN_FILE).unwrap();
                login_page(root);
            })
        );
    } else {
        welcome_page(&mut root);
    }

    //start the event loop
    root.run();
}
