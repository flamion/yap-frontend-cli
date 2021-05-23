use cursive;
use cursive::align::HAlign;
use cursive::Cursive;
use cursive::event::Key;
use cursive::theme::ColorStyle;
use cursive::view::{Nameable, Resizable, SizeConstraint};
use cursive::views::{
    Dialog, EditView, LinearLayout, TextView, Checkbox,
    SelectView, ScrollView, ResizedView, Layer, StackView, Panel, Button
};
use regex::Regex;
use std::ops::Not;
use reqwest::blocking;
use reqwest::StatusCode;
use std::io::Write;
use std::fs;
use std::vec;
use std::fs::{File, OpenOptions};
use serde_json;
use std::path::{Path, PathBuf};
use serde::Serialize;
use serde::Deserialize;
use xdg;
use chrono;
//use std::thread;
//use std::sync::mpsc;




//TODO rewrite login so it takes email and password as arguments

//TOKEN_FILE name
static TOKEN_FILE: &'static str = "token.json";

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct BoardAPI {
    boardID: i64,
    name: String,
    createDate: i64,
    creatorID: i64, //UserID
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct UserAPI {
    userID: i64,
    username: String,
    createDate: i64,
    lastLogin: i64,
    emailAddress: String,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct EntryAPI {
    entryID: i64,
    creatorID: i64,
    createDate: i64,
    dueDate: i64,
    title: String,
    description: String,
}

struct Board {
    board_id: i64,
    name: String,
    create_date: chrono::DateTime<chrono::offset::Local>,
    creator_id: i64,
}

struct User {
    user_id: i64,
    name: String,
    create_date: chrono::DateTime<chrono::offset::Local>,
    last_login: chrono::DateTime<chrono::offset::Local>,
    email_address: String,
}

struct Entry {
    entry_id: i64,
    creator_id: i64,
    create_date: chrono::DateTime<chrono::offset::Local>,
    due_date: chrono::DateTime<chrono::offset::Local>,
    title: String,
    description: String,
}

enum EntryItem {
    Entry(Entry),
    Add,
}

enum BoardItem {
    Board(Board),
    Add,
}

enum BackendError {
    BoardDeleted, //204
    TokenInvalid, //401
    NoAccess, //403
}

struct Tab {
    indicator: &'static str,
    layer: &'static str,
}

//Used to "emulate" enums, as Cursive only supports Strings as names
static TABS: [Tab;2] = [
    Tab {
        indicator: "BOARDS_TAB_INDICATOR",
        layer: "BOARDS_TAB_LAYER",
    },
    Tab {
        indicator: "ACCOUNT_TAB_INDICATOR",
        layer: "ACCOUNT_TAB_LAYER",
    }
];

struct GlobalData {
    http_client: blocking::Client,
    token: Option<String>,
    config_home: xdg::BaseDirectories,
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



fn set_entry_nav_callback(siv: &mut Cursive) {
    siv.add_global_callback(
        Key::Esc,
        |s| {
            switch_stack(
                s,
                "BOARD_STACK",
                "BOARD_LAYER"
            );
            clear_entry_view(s);
        }
    );
}

fn select_tab(siv: &mut Cursive, tab_name: &Tab) {
    for tab in &TABS {
        let mut tab_indicator = siv.find_name::<Layer<TextView>>(tab.indicator)
            .expect("tab indicator not found");

        if tab.indicator.eq(tab_name.indicator) {
            tab_indicator.set_color(ColorStyle::highlight());
            tab_indicator.get_inner_mut().set_style(ColorStyle::highlight());
        } else {
            tab_indicator.set_color(ColorStyle::primary());
            tab_indicator.get_inner_mut().set_style(ColorStyle::primary());
        }
    }

    let mut tab_layers = siv.find_name::<StackView>("TAB_LAYERS")
        .expect("tab layers not found");

    let tab_layer = tab_layers.find_layer_from_name(tab_name.layer)
        .expect("tab layer not found");

    tab_layers.move_to_front(tab_layer);

    siv.clear_global_callbacks(Key::Esc);

    if tab_name.layer == TABS[0].layer {
        set_entry_nav_callback(siv);
    }
}

/*fn reload_all(siv: &mut Cursive) {

}*/

fn load_boards_to_view(siv: &mut Cursive) {
    match get_board_ids(siv) {
        Ok(board_ids) => {
            for board_id in board_ids {
                match get_board_from_id(siv, board_id) {
                    Ok(board_obj) => {
                        load_to_board_view(siv, board_obj);
                    },
                    Err(error) => {
                        match error {
                            BackendError::BoardDeleted => notify_popup(siv, "board deleted", "board doesn't exist anymore"),
                            BackendError::TokenInvalid => notify_popup(siv, "session invalid", "please re-login"),
                            BackendError::NoAccess => panic!("received no access: 403"),
                        }
                    },
                }
            }
        },
        Err(error) => {
            match error {
                BackendError::BoardDeleted => notify_popup(siv, "board deleted", "board doesn't exist anymore"),
                BackendError::TokenInvalid => notify_popup(siv, "session invalid", "please re-login"),
                BackendError::NoAccess => panic!("received no access: 403"),
            }
        },
    }
}

fn load_entries_to_view(siv: &mut Cursive, board_id: i64) {
    match get_board_entry_ids(siv, board_id) {
        Ok(entry_ids) => {
            for entry_id in entry_ids {
                match get_entry_from_id(siv, entry_id) {
                    Ok(entry_obj) => {
                        load_to_entry_view(siv, entry_obj);
                    },
                    Err(error) => {
                        match error {
                            BackendError::BoardDeleted => notify_popup(siv, "entry deleted", "entry doesn't exist anymore"),
                            BackendError::TokenInvalid => notify_popup(siv, "session invalid", "please re-login"),
                            BackendError::NoAccess => panic!("received no access: 403"),
                        }
                    },
                }
            }
        },
        Err(error) => {
            match error {
                BackendError::BoardDeleted => notify_popup(siv, "entry deleted", "entry doesn't exist anymore"),
                BackendError::TokenInvalid => notify_popup(siv, "session invalid", "please re-login"),
                BackendError::NoAccess => panic!("received no access: 403"),
            }
        },
    }
}

fn on_submit_board(siv: &mut Cursive, item: &BoardItem) {
    match item {
        BoardItem::Board(board) => {
            load_entries_to_view(siv, board.board_id);
            switch_stack(siv, "BOARD_STACK", "ENTRY_LAYER");
        },
        BoardItem::Add => (),
    }
}

fn on_submit_entry(siv: &mut Cursive, item: &EntryItem) {
    match item {
        EntryItem::Entry(entry) => {
            notify_popup(siv, "edit entry", "edit this entry");
        },
        EntryItem::Add => (),
    }
}

fn on_select_entry(siv: &mut Cursive, item: &EntryItem) {
    match item {
        EntryItem::Entry(entry) => {
            siv.find_name::<TextView>("ENTRY_DESCRIPTION")
                .expect("view: 'ENTRY_DESCRIPTION' not found")
                .set_content(entry.description.clone());
        },
        EntryItem::Add => {
            siv.find_name::<TextView>("ENTRY_DESCRIPTION")
                .expect("view: 'ENTRY_DESCRIPTION' not found")
                .set_content("");
        },
    }
}

fn entry_api_to_entry(entry_api: EntryAPI) -> Entry {
    return Entry {
        entry_id: entry_api.entryID,
        creator_id: entry_api.creatorID,
        create_date: chrono::DateTime::from(
            chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(
                    entry_api.createDate, 0
                ),
                chrono::Utc
            )
        ),
        due_date: chrono::DateTime::from(
            chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(
                    entry_api.dueDate, 0
                ),
                chrono::Utc
            )
        ),
        title: entry_api.title,
        description: entry_api.description,
    };
}

fn board_api_to_board(board_api: BoardAPI) -> Board {
    return Board {
        board_id: board_api.boardID,
        name: board_api.name,
        create_date: chrono::DateTime::from(
            chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(
                    board_api.createDate, 0
                ),
                chrono::Utc
            )
        ),
        creator_id: board_api.creatorID,
    };
}

fn clear_entry_view(siv: &mut Cursive) {
    let mut entry_view = siv.find_name::<SelectView<EntryItem>>("ENTRY_SELECTION")
        .expect("view: 'ENTRY_SELECTION' not found");

    entry_view.clear();
    entry_view.add_item("<add new entry>", EntryItem::Add);
}

fn load_to_entry_view(siv: &mut Cursive, entry: Entry) {
    siv.find_name::<SelectView<EntryItem>>("ENTRY_SELECTION")
        .expect("view: 'ENTRY_SELECTION' not found")
        .insert_item(0, entry.title.clone(), EntryItem::Entry(entry));
}

fn switch_stack(siv: &mut Cursive, stack_name: &str, layer_name: &str) {
    let mut stack = siv.find_name::<StackView>(stack_name) // = "BOARD_STACK"
        .expect("view not found");

    let stack_position = stack.find_layer_from_name(layer_name)
        .expect("layer not found"); // = "ENTRY_LAYER"

    stack.move_to_front(stack_position);
}

fn load_to_board_view(siv: &mut Cursive, board: Board) {
    siv.find_name::<SelectView<BoardItem>>("BOARD_SELECTION")
        .expect("view: 'BOARD_SELECTION' not found")
        .insert_item(0, board.name.clone(), BoardItem::Board(board));
}

fn error_handler(status_code: reqwest::StatusCode) -> BackendError {
    if status_code == StatusCode::UNAUTHORIZED {
        return BackendError::TokenInvalid;
    } else if status_code == StatusCode::NO_CONTENT {
        return BackendError::BoardDeleted;
    } else {
        panic!("server returned an unexpected status");
        return BackendError::NoAccess;
    }
}

fn get_board_from_id(siv: &mut Cursive, board_id: i64) -> Result<Board, BackendError>{
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("https://backend.yap.dragoncave.dev/boards/{}", board_id))
        .header("token", token)
        .send() {

        Ok(response) => if response.status().is_success() && response.status() != StatusCode::NO_CONTENT {
            return Ok(
                board_api_to_board(
                    response.json::<BoardAPI>()
                        .expect("didn't receive matching json object")
                )
            );
        } else {
            return Err(error_handler(response.status()))
        }
        Err(_) => panic!("request went in error path"),
    }
}

fn get_board_entry_ids(siv: &mut Cursive, board_id: i64) -> Result<vec::Vec<i64>, BackendError> { //board 8
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("https://backend.yap.dragoncave.dev/boards/{}/entries", board_id))
        .header("token", token).send() {
        Ok(response) => if response.status().is_success() {
            return Ok(response.json::<vec::Vec<i64>>().expect("didn't receive json array of i64's"));
        } else {
            return Err(error_handler(response.status()))
        },
        Err(_) => panic!("request went in error path")
    }
}

fn get_entry_from_id(siv: &mut Cursive, entry_id: i64) -> Result<Entry, BackendError> {
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("https://backend.yap.dragoncave.dev/entry/{}", entry_id))
        .header("token", token)
        .send() {

        Ok(response) => if response.status().is_success() && response.status() != StatusCode::NO_CONTENT {
            return Ok(
                entry_api_to_entry(
                    response.json::<EntryAPI>()
                        .expect("didn't receive matching json object")
                )
            );
        } else {
            return Err(error_handler(response.status()))
        }
        Err(_) => panic!("request went in error path"),
    }
}

fn get_board_ids(siv: &mut Cursive) -> Result<vec::Vec<i64>, BackendError> {
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get("https://backend.yap.dragoncave.dev/boards/user")
        .header("token", token).send() {
        Ok(response) => if response.status().is_success() {
            return Ok(response.json::<vec::Vec<i64>>().expect("didn't receive json array of i64's"));
        } else {
            return Err(error_handler(response.status()))
        },
        Err(_) => panic!("request went in error path")
    }
}

fn exit(siv: &mut Cursive) {
    siv.quit();
}

fn login_page(siv: &mut Cursive) {
    siv.pop_layer();
    siv.add_layer(Dialog::new()
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
                        .on_change(|siv, state|
                            if let Some(mut view) =
                            siv.find_name::<EditView>("PASSWORD_LOGIN") {
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
        .button("Back", |siv| welcome_page(siv))
        .button("Login", login)
    );
}

fn login(siv: &mut Cursive) {

    let email = siv.find_name::<EditView>("EMAIL_LOGIN")
        .unwrap_or_else(
            || siv.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name"))
        .get_content();

    let password = siv.find_name::<EditView>("PASSWORD_LOGIN")
        .unwrap_or_else(
            || siv.find_name::<EditView>("PASSWORD_REGISTER")
                .expect("couldn't find view by name"))
        .get_content();

    //let config_dir = &siv.user_data::<GlobalData>()
    //    .unwrap()
    //    .config_home;

    //Get HTTP client if it exists else create one and store it for later use
    /*let http_client = siv.take_user_data::<GlobalData>().unwrap_or(
        GlobalData {
            http_client: blocking::Client::new(),
            //token: "".to_string(),
            token: None,
        }).http_client;*/


    //file.unwrap().write_all(password.as_bytes()).unwrap();
    //let mut filee = File::create(siv.user_data::<GlobalData>().unwrap().config_home.find_data_file(TOKEN_FILE).expect("file not found")).expect("file wasn't created");
    //let mut file = File::create("reached");

    //Send request to backend to obtain a token
    match siv.user_data::<GlobalData>().expect("no user data set").http_client.post("https://backend.yap.dragoncave.dev/security/token")
        .header("content-type", "application/json")
        .body(format!(
            "{{\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            email,
            password
        ))
        .send() {

        Ok(request) => { // <- If the status code is an Error it will still return an Ok()
            if request.status().is_success() {
                remove_file(siv, TOKEN_FILE);

                siv.with_user_data(|data: &mut GlobalData | {
                    data.token = Some(request.text().unwrap());
                });

                //Write the token to a file if REMEMBER_ME is checked
                if let Some(state) = siv.find_name::<Checkbox>(
                    "REMEMBER_ME_LOGIN"
                ) {
                    if state.is_checked().eq(&true) {

                        create_file(siv, TOKEN_FILE);

                        if let Ok(mut file) = get_file(siv, TOKEN_FILE) {
                            file.write_all(
                                serde_json::to_string_pretty(
                                    &TokenFile {
                                        user_mail: email.to_string(),
                                        token: siv.user_data::<GlobalData>()
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
                            notify_popup(siv, "Remember Me", "Remember Me didn't work :(")
                        }
                    }
                } else {
                    notify_popup(siv, "No success!", "file not created");
                }
                main_screen(siv);
            } else {
                if request.status() == StatusCode::FORBIDDEN {
                    notify_popup(
                        siv,
                        "Wrong credentials!",
                        "Either your mail or password is wrong."
                    );
                } else {
                    notify_popup(
                        siv,
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
                notify_popup(siv, "Request failed.",
                             &*format!("Request failed with code: {}", status))
            } else {
                notify_popup(siv, "Request failed.", "Reason: Unknown");
            }
        },
    }
}


fn main_screen(siv: &mut Cursive) {
    siv.pop_layer();

    siv.add_fullscreen_layer(
        LinearLayout::vertical()
            .child(
                ResizedView::new(
                    SizeConstraint::Full,
                    SizeConstraint::Fixed(1),
                    LinearLayout::horizontal()
                        .child(
                            ResizedView::with_full_screen(
                                Layer::with_color(
                                    TextView::new("Boards")
                                        .h_align(HAlign::Center)
                                        .style(ColorStyle::highlight()),
                                    ColorStyle::highlight()
                                ).with_name(TABS[0].indicator)
                            )
                        )
                        .child(
                            ResizedView::with_full_screen(
                                Layer::new(
                                    TextView::new("Account")
                                        .h_align(HAlign::Center)
                                ).with_name(TABS[1].indicator)
                            )
                        )
                )
            )
            .child(
                LinearLayout::vertical()
                    .child(
                        StackView::new()
                            .fullscreen_layer(
                                ResizedView::with_full_screen(
                                    LinearLayout::vertical()
                                        .child(
                                            TextView::new("Password")
                                        )
                                        .child(
                                            EditView::new()
                                        )
                                ).with_name(TABS[1].layer)
                            )
                            .fullscreen_layer(
                                StackView::new()
                                    .fullscreen_layer(
                                        LinearLayout::horizontal()
                                            .child(
                                                ResizedView::with_full_screen(
                                                    ScrollView::new(
                                                        ResizedView::with_full_screen(
                                                            SelectView::new()
                                                                .autojump()
                                                                .on_submit(on_submit_entry)
                                                                .on_select(on_select_entry)
                                                                .with_name("ENTRY_SELECTION")
                                                        )
                                                    )
                                                )
                                            )
                                            .child(
                                                ResizedView::with_full_screen(
                                                    Panel::new(
                                                        ScrollView::new(
                                                            TextView::new("TextText on new line")
                                                                .with_name("ENTRY_DESCRIPTION")
                                                        )
                                                    ).title("Description")
                                                        .title_position(HAlign::Center)
                                                )
                                            ).with_name("ENTRY_LAYER")
                                    )
                                    .fullscreen_layer(
                                        ResizedView::with_full_screen(
                                            ScrollView::new(
                                                ResizedView::with_full_screen(
                                                    SelectView::new()
                                                        .autojump()
                                                        .item("<add new board>", BoardItem::Add)
                                                        .on_submit(on_submit_board)
                                                        .with_name("BOARD_SELECTION")
                                                )
                                            ).with_name(TABS[0].layer)
                                        ).with_name("BOARD_LAYER")
                                    ).with_name("BOARD_STACK")
                            ).with_name("TAB_LAYERS")

                    )
                    /*.child(
                        ResizedView::new(
                            SizeConstraint::Full,
                            SizeConstraint::Fixed(1),
                            Button::new("reload", reload_all)
                        )
                    )*/
            )
    );

    siv.add_global_callback(Key::Left, |siv| select_tab(siv, &TABS[0]));
    siv.add_global_callback(Key::Right, |siv| select_tab(siv, &TABS[1]));

    clear_entry_view(siv);

    set_entry_nav_callback(siv);

    load_boards_to_view(siv);
}

fn register_page(siv: &mut Cursive) {

    siv.pop_layer();
    siv.add_layer(Dialog::new()
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
                    .on_change(|siv, state|
                        if let Some(mut view) =
                        siv.find_name::<EditView>("PASSWORD_REGISTER") {
                            if let Some(mut check_view) =
                            siv.find_name::<EditView>("PASSWORD_CHECK_REGISTER") {

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
        .button("Back", |siv| welcome_page(siv))
        .button("Register and login", |siv| {
                match check_register(siv) {
                    Ok(_) => register(siv),
                    Err(RegisterInvalid::InvalidUsername) =>
                        notify_popup(siv, "credentials not valid",
                                     "error: username not valid"),
                    Err(RegisterInvalid::InvalidEmail) =>
                        notify_popup(siv, "credentials not valid",
                                     "error: email not valid"),
                    Err(RegisterInvalid::InvalidPassword) =>
                        notify_popup(siv, "credentials not valid",
                                     "error: password not valid"),
            }
        })
    );
}

fn notify_popup(siv: &mut Cursive, title: &str, message: &str) {
    siv.add_layer(
        Dialog::text(message)
            .title(title)
            .dismiss_button("Ok")
    );
}

fn check_register(siv: &mut Cursive) -> Result<(), RegisterInvalid> {
    let username = siv.find_name::<EditView>("USERNAME_REGISTER")
        .expect("couldn't find view by name");

    let email = siv.find_name::<EditView>("EMAIL_REGISTER")
        .expect("couldn't find view by name");

    let password = siv.find_name::<EditView>("PASSWORD_REGISTER")
        .expect("couldn't find view by name");

    let password_check = siv.find_name::<EditView>("PASSWORD_CHECK_REGISTER")
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

fn register(siv: &mut Cursive) {
    match siv.user_data::<GlobalData>().expect("no user data set").http_client.post("https://backend.yap.dragoncave.dev/user")
        .header("content-type", "application/json")
        .body(format!(
            "{{\"username\":\"{}\",\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            siv.find_name::<EditView>("USERNAME_REGISTER")
                .expect("couldn't find view by name").
                get_content(),
            siv.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name")
                .get_content(),
            siv.find_name::<EditView>("PASSWORD_REGISTER")
                .expect("couldn't find view by name")
                .get_content()
        ))
        .send() {
        Ok(_) => {
            notify_popup(siv, "Success!", "Successfully created user");
        },
        Err(error) => {
            if let Some(status) = error.status() {
                notify_popup(siv, "Request failed.",
                             &*format!("Request failed with code: {}", status))
            } else {
                notify_popup(siv, "Request failed.", "Reason: Unknown");
            }
        },
    }
    siv.pop_layer();
    login(siv);
}

fn get_path(siv: &mut Cursive, file: &str) -> Result<PathBuf, std::io::ErrorKind> {
    if let Some(path) = &siv.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .find_config_file(file) {
        return Ok(path.clone());
    } else {
        return Err(std::io::ErrorKind::NotFound);
    }
}

fn get_file(siv: &mut Cursive, file: &str) -> Result<File, std::io::ErrorKind> {
    if let Ok(path) = get_path(siv, file) {
        return Ok(OpenOptions::new()
            .write(true)
            .open(path)
            .expect("file couldn't be opened"));
    } else {
        return Err(std::io::ErrorKind::NotFound);
    }
}

//TODO add Result<(), Error>
fn remove_file(siv: &mut Cursive, file: &str) {
    if let Some(file_path) = &siv.user_data::<GlobalData>()
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
fn create_file(siv: &mut Cursive, file: &str) {
    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .place_config_file(file) {
        Ok(file_path) => if Path::exists(file_path.as_ref()).not() {
            File::create(file_path).expect("couldn't create file");
        }
        Err(error) => panic!("{}", error.to_string()),
    }
}

fn welcome_page(siv: &mut Cursive) {
    siv.pop_layer();
    siv.add_layer(Dialog::text(
        "Welcome to YAP!\nPress <Login> if you already have an account, \
         else consider creating one by pressing <Register>"
    )
        .title("Login - YAP")
        .button("Quit", |siv| siv.quit())
        .button("Login", login_page)
        .button("Register", register_page));
}

fn check_token(siv: &mut Cursive, token: &str) -> bool {
    if let Ok(response) = siv.user_data::<GlobalData>().expect("no user data set").http_client.get("https://backend.yap.dragoncave.dev/security/token/checkValid")
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
fn load_token(siv: &mut Cursive) -> Result<TokenFile, TokenLoadError> {
    if let Ok(path) = get_path(siv, TOKEN_FILE) {
        if let Ok(token_content) = fs::read_to_string(path) {
            if let Ok(token_struct) = serde_json::from_str::<TokenFile>(&*token_content) {
                if check_token(siv, &token_struct.token) {
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
    let mut siv = cursive::default();

    //bind exit to 'q' to be able to exit at any time
    siv.add_global_callback('q', exit);

    siv.add_global_callback('\\', Cursive::toggle_debug_console);

    siv.set_user_data(GlobalData {
        token: None,
        http_client: blocking::Client::new(),
        config_home: xdg::BaseDirectories::with_prefix("yap").unwrap(),
    });

    //load theme file if present
    if let Ok(file) = get_path(&mut siv, "theme.toml") {
        siv.load_theme_file(file).unwrap();
    }

    //siv.user_data::<GlobalData>().unwrap().config_home.place_data_file(TOKEN_FILE).expect("token file not placed");
    //siv.with_user_data(|data: &mut GlobalData| data.config_home.place_data_file(TOKEN_FILE).expect("couldn't place token file"));

    //display the welcome page
    //TODO integrate this mess into the login function, which should by then use the credentials as parameters
    if let Ok(token_comb) = load_token(&mut siv) {
        siv.add_layer(Dialog::text(
            format!("Is {} you?", token_comb.user_mail))
            .button("yes", move |mut siv| {

                //let http_client = blocking::Client::new();

                /*siv.set_user_data(
                    GlobalData {
                        token: Some(token_comb.token.clone()),
                        http_client,

                    }
                );*/

                siv.with_user_data(|data: &mut GlobalData| {
                    data.token = Some(token_comb.token.clone());
                });

                main_screen(&mut siv);
            })
            .button("no", |siv| {
                remove_file(siv, TOKEN_FILE);
                login_page(siv);
            })
        );
    } else {
        welcome_page(&mut siv);
    }

    //start the event loop
    siv.run();
}
