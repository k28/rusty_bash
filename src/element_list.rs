//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

#[derive(PartialEq, Clone, Debug)]
pub enum ControlOperator {
    Or, /* || */
    And, /* && */
    BgAnd, /* & */
    Semicolon, /* ; */
    DoubleSemicolon, /* ;; */
    SemiAnd, /* ;& */
    SemiSemiAnd, /* ;;& */
    Pipe, /* | */
    PipeAnd, /* |& */
    NewLine, /* \n */
    NoChar,
}