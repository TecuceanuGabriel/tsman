#[derive(Debug)]
pub enum MenuAction {
    Open,
    Delete,
    Edit,
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
