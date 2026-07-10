import { test } from 'node:test';
import assert from 'node:assert/strict';
import { say, voiceFor, BOOT } from './voice.ts';

// Narration axis must actually change the voice — plain/corporate-killer forbid wit everywhere.
test('GREEN: plain narration forces every line to plain tone (no wit)', () => {
  for (const key of ['404', 'save.success', 'generic.error', 'sacred.footer']) {
    assert.equal(voiceFor('plain').say(key).tone, 'plain', `plain axis must strip wit on ${key}`);
  }
});

test('GREEN: bebop narration keeps brand wit on brand moments', () => {
  assert.equal(voiceFor('bebop').say('404').tone, 'brand', 'bebop keeps wit on 404');
  assert.equal(voiceFor('bebop').say('payment.failed').tone, 'plain', 'money stays plain under bebop');
});

test('GREEN: boot lines differ per narration (init actually changes the voice)', () => {
  assert.notEqual(BOOT.bebop.ready, BOOT.sarcastic.ready, 'bebop vs sarcastic boot must differ');
  assert.notEqual(BOOT.bebop.ready, BOOT.plain.ready, 'bebop vs plain boot must differ');
  assert.ok(BOOT.plain.ready.length > 0, 'plain boot line must exist');
});

test('GREEN: default (no axis) behaves like bebop', () => {
  assert.equal(say('404').tone, 'brand', 'default narration keeps brand wit');
  assert.equal(voiceFor(undefined).say('404').tone, 'brand', 'undefined axis = bebop');
});
