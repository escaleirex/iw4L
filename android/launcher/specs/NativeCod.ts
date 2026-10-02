import type {TurboModule, CodegenTypes} from 'react-native';
import {TurboModuleRegistry} from 'react-native';

export interface Spec extends TurboModule {
  getInstallation(): Promise<string>;
  selectInstallation(): Promise<string>;
  validateInstallation(): Promise<string>;
  startRuntime(diagnostic: boolean): Promise<void>;
  readonly onControllerAction: CodegenTypes.EventEmitter<string>;
}

export default TurboModuleRegistry.getEnforcing<Spec>('NativeCod');
