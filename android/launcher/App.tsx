import React, {useCallback, useEffect, useRef, useState} from 'react';
import {AppState, BackHandler, Pressable, ScrollView, StatusBar, StyleSheet, Text, View} from 'react-native';
import NativeCod from './specs/NativeCod';
import {games, type Installation} from './games';

export default function App() {
  const [page, setPage] = useState<'home' | 'mw2'>('home');
  const [focus, setFocus] = useState(0);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [installation, setInstallation] = useState<Installation>({configured: false, valid: false});
  const scroll = useRef<React.ElementRef<typeof ScrollView>>(null);
  const buttonPositions = useRef<Record<number, number>>({});
  const viewportHeight = useRef(0);
  const refresh = useCallback(async () => {
    try { setInstallation(JSON.parse(await NativeCod.getInstallation())); }
    catch (e) { setMessage(String(e)); }
  }, []);
  useEffect(() => {
    void refresh();
    const subscription = AppState.addEventListener('change', state => { if (state === 'active') void refresh(); });
    return () => subscription.remove();
  }, [refresh]);
  const goBack = useCallback(() => {
    if (page === 'mw2') { setPage('home'); setFocus(0); return true; }
    return false;
  }, [page]);
  useEffect(() => { const sub = BackHandler.addEventListener('hardwareBackPress', goBack); return () => sub.remove(); }, [goBack]);
  const action = useCallback(async (index: number) => {
    if (busy) return;
    if (page === 'home') {
      if (index === 0) { setPage('mw2'); setFocus(0); setMessage(''); }
      return;
    }
    setBusy(true); setMessage('');
    try {
      if (index === 0) setInstallation(JSON.parse(await NativeCod.selectInstallation()));
      if (index === 1) setInstallation(JSON.parse(await NativeCod.validateInstallation()));
      if (index === 2) await NativeCod.startRuntime(false);
      if (index === 3) await NativeCod.startRuntime(true);
    } catch (e) { setMessage(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  }, [busy, page]);
  useEffect(() => {
    const sub = NativeCod.onControllerAction(command => {
      if (command === 'back') { if (!goBack()) BackHandler.exitApp(); return; }
      if (command === 'confirm') { if (page !== 'mw2' || focus !== 2 || installation.valid) void action(focus); return; }
      const delta = command === 'up' || command === 'left' ? -1 : 1;
      const next = Math.max(0, Math.min((page === 'home' ? games.length : 4) - 1, focus + delta));
      setFocus(next);
      const position = page === 'home' ? next * 90 : buttonPositions.current[next] ?? 0;
      scroll.current?.scrollTo({y: Math.max(0, position - viewportHeight.current / 2 + 30), animated: true});
    });
    return () => sub.remove();
  }, [action, focus, goBack, installation.valid, page]);
  const button = (label: string, index: number, disabled = false) => (
    <Pressable accessibilityRole="button" accessibilityState={{disabled}} disabled={disabled || busy}
      onLayout={event => { buttonPositions.current[index] = event.nativeEvent.layout.y; }}
      onPress={() => { setFocus(index); void action(index); }}
      style={[styles.button, focus === index && styles.focus, disabled && styles.disabled]}>
      <Text style={styles.buttonText}>{label}</Text>
    </Pressable>
  );
  return <View style={styles.root}>
    <StatusBar hidden />
    <View style={styles.sidebar}>
      <Text style={styles.brand}>NATIVE<Text style={styles.accent}> COD</Text></Text>
      <Text style={styles.eyebrow}>YOUR LIBRARY</Text>
      <Text style={styles.sidebarTitle}>{page === 'home' ? 'Classic games.\nA new home.' : 'Modern\nWarfare 2'}</Text>
      <Text style={styles.muted}>ARM64 Native · Android</Text>
      <View style={styles.footer}><Text style={styles.muted}>D-pad  Navigate</Text><Text style={styles.muted}>A  Select     B  Back</Text></View>
    </View>
    <View style={styles.content}>
      {page === 'home' ? <>
        <Text style={styles.heading}>Library</Text>
        <ScrollView ref={scroll} onLayout={event => { viewportHeight.current = event.nativeEvent.layout.height; }} contentContainerStyle={styles.list}>
          {games.map((game, index) => <Pressable key={game.id} accessibilityRole="button"
            onPress={() => { setFocus(index); if (game.runtime) { setPage('mw2'); setFocus(0); } }}
            style={[styles.card, focus === index && styles.focus]}>
            <View><Text style={styles.cardTitle}>{game.title}</Text><Text style={styles.muted}>{game.year}</Text></View>
            <Text style={game.runtime ? styles.accent : styles.muted}>{game.runtime ? (installation.valid ? 'Installation detected' : 'Not configured') : 'Coming soon'}</Text>
          </Pressable>)}
        </ScrollView>
      </> : <ScrollView ref={scroll} onLayout={event => { viewportHeight.current = event.nativeEvent.layout.height; }} contentContainerStyle={styles.detail}>
        <Text style={styles.eyebrow}>IW4 · 2009</Text><Text style={styles.heading}>Modern Warfare 2</Text>
        <Text style={styles.muted}>Runtime: IW4     Architecture: ARM64 Native     Renderer: Vulkan</Text>
        <View style={styles.installation}><Text style={styles.cardTitle}>Game files</Text>
          <Text style={installation.valid ? styles.accent : styles.muted}>{installation.valid ? 'Installation detected ✓' : installation.configured ? 'Needs attention' : 'Not configured'}</Text>
          {!!installation.name && <Text style={styles.muted}>{installation.name}</Text>}
          {!!installation.error && <Text style={styles.error}>{installation.error}</Text>}
        </View>
        {button('Select Game Installation', 0)}
        {button('Validate game files', 1, !installation.configured)}
        {button('PLAY', 2, !installation.valid)}
        {button('Check Vulkan', 3)}
        <Text style={styles.muted}>Development build · Multiplayer runtime integration in progress.</Text>
        {!!installation.runtimeStatus && <Text style={styles.muted}>{installation.runtimeStatus}</Text>}
      </ScrollView>}
      {busy && <Text style={styles.notice}>Checking game files…</Text>}
      {!!message && <Text accessibilityRole="alert" style={styles.error}>{message}</Text>}
    </View>
  </View>;
}
const styles = StyleSheet.create({
  root: {flex: 1, flexDirection: 'row', backgroundColor: '#101513'},
  sidebar: {width: '29%', backgroundColor: '#171f1b', padding: 28},
  brand: {fontSize: 21, fontWeight: '900', color: '#f2f5ef', marginBottom: 42},
  accent: {color: '#c5e979', fontWeight: '600'},
  eyebrow: {fontSize: 11, letterSpacing: 2, color: '#a3ad9d', marginBottom: 12},
  sidebarTitle: {fontSize: 30, lineHeight: 37, color: '#f2f5ef', fontWeight: '700', marginBottom: 20},
  muted: {color: '#a7b1a9', fontSize: 12, lineHeight: 20},
  footer: {marginTop: 'auto', paddingTop: 22},
  content: {flex: 1, padding: 24},
  heading: {fontSize: 29, fontWeight: '700', color: '#f2f5ef', marginBottom: 16},
  list: {gap: 10, paddingBottom: 8},
  card: {minHeight: 80, padding: 15, borderWidth: 2, borderColor: '#27332c', borderRadius: 8, backgroundColor: '#19211c', flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between'},
  cardTitle: {fontSize: 18, color: '#f2f5ef', fontWeight: '600', marginBottom: 4},
  focus: {borderColor: '#c5e979', backgroundColor: '#2a3623'},
  detail: {gap: 10, paddingBottom: 18},
  installation: {paddingVertical: 12},
  button: {padding: 13, borderWidth: 2, borderColor: '#364337', borderRadius: 6, backgroundColor: '#202b22'},
  buttonText: {color: '#f2f5ef', fontSize: 15, fontWeight: '700'},
  disabled: {opacity: 0.4},
  notice: {color: '#c5e979', paddingTop: 8},
  error: {color: '#ffb4a9', fontSize: 13, lineHeight: 20, paddingTop: 6},
});
