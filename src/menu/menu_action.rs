#[derive(Debug)]
pub enum MenuAction {
    Open,
    Delete,
    Edit,
    Save,
    Kill,
    MoveSelection(i32),
    TogglePreview,
    ToggleHelp,
    RemoveLastWord,
    ShowConfirmation,
    HideConfirmation,
    Exit,
}
