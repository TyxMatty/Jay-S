// Failures from the study commands (src-tauri/src/study.rs) in the student's language.
import { t } from '../core/i18n.js';
import { failureText } from './materials.js';
import { aiFailureText } from './jayai.js';

export function studyFailureText(error) {
  switch (error?.code) {
    case 'busy':
      return t('Jay-S is already working on something: wait for it to finish (or stop it).');
    case 'empty':
      return t('There was nothing to make it from: pick materials with text, or an exam whose questions Jay-S found.');
    case 'unknown_style':
      return t('Unknown exam style.');
    case 'finished':
      return t('This attempt has already ended.');
    case 'not_finished':
      return t('This attempt is still going.');
    case 'time_up':
      return t("Time's up.");
    case 'locked':
      return t('This question was already checked.');
    case 'invalid_answer':
      return t('That answer is not one of the options.');
    case 'too_big':
    case 'unsupported':
    case 'invalid':
    case 'not_found':
    case 'web':
      return failureText(error);
    default:
      return aiFailureText(error);
  }
}
