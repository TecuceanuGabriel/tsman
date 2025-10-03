#[derive(Debug)]
pub enum MenuAction {
    Open,
    Delete,
    Edit,
    Rename,
    Save,
    Kill,
    MoveSelection(i32),
    AppendToInput(char),
    DeleteFromInput,
    RemoveLastWord,
    TogglePreview,
    ToggleHelp,
    HideConfirmation,
    Exit,
    Nop,
}
